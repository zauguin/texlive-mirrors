use anyhow::{bail, Error};
use nom::Parser;
use parse_ctanmirrors::{Mirror, Mirrors};
use reqwest;
use serde::Serialize;
use serde_json;
use std::{collections::{HashMap, HashSet}, io::Read, sync::Arc, time::Duration, env};
use tokio::{
    sync::{watch, Notify},
    task::JoinSet,
    time::timeout,
};
use xz::read::XzDecoder;
use sha2::{Digest, Sha512};
use hex;
use schemars::{schema_for, JsonSchema};

mod parse_ctanmirrors;
mod parse_tlpdb;

async fn parse_mirrors() -> Result<Mirrors, Error> {
    let response = reqwest::get(
        "https://ctan.math.hamburg/systems/texlive/tlnet/tlpkg/installer/ctan-mirrors.pl",
    )
    .await?
    .text()
    .await?;

    let input = response.as_str();
    let (rest, result) = parse_ctanmirrors::parse_mirrors::<nom::error::Error<_>>()
        .parse(input)
        .map_err(|err| err.map_input(|i| i.to_owned()))?;
    if !rest.is_empty() {
        bail!("Unexpected garbage after mirror list");
    }

    Ok(result.into())
}

/// # Mirror status
#[derive(Debug, Serialize, Clone, JsonSchema)]
#[serde(tag = "status")]
enum MirrorData {
    /// # Dead
    /// Mirror is unusable for some reason. This is used all general collections
    /// for miscellaneous reasons why a mirror might be unusable.
    /// Unknown values should be treated like `Dead'.
    Dead,
    /// # Alive
    /// Usable mirror with the specified TeX Live revision.
    Alive { texlive_version: u16, revision: u32 },
    /// # Timeout
    /// Mirror did not answer in a reasonable time.
    Timeout,
    /// # Excluded
    /// Mirror is known to be unstable and therefore manually excluded.
    Excluded,
}

/// # Map from mirror URL to mirror status
#[derive(Debug, Serialize, JsonSchema)]
struct CountryMirrorsWithData(HashMap<Mirror, MirrorData>);
/// # Map from country to corresponding mirrors
#[derive(Debug, Serialize, JsonSchema)]
struct ContinentMirrorsWithData(HashMap<String, CountryMirrorsWithData>);
/// # Map from continents to corresponding mirrors
#[derive(Debug, Serialize, JsonSchema)]
struct MirrorsWithData(HashMap<String, ContinentMirrorsWithData>);

fn get_mirror_data_from_tlpdb(tlpdb_text: &str) -> MirrorData {
    let Ok((remaining, parsed)) = parse_tlpdb::parse_entries::<nom::error::VerboseError<_>>()
        .parse(tlpdb_text) else {
            return MirrorData::Dead
        };
    if !remaining.is_empty() {
        return MirrorData::Dead
    }
    let Some(first_entry) = parsed.get(0) else {
        return MirrorData::Dead
    };
    if first_entry.name != "00texlive.config" {
        eprintln!("Invalid name of first entry: {}", first_entry.name);
        return MirrorData::Dead
    }
    let dependencies = &first_entry.depend;
    let mut found_release = None;
    let mut found_revision = None;
    for dependency in dependencies {
        if let Some(release) = dependency.strip_prefix("release/") {
            if let Ok(release) = release.parse() {
                found_release = Some(release);
            } else {
                return MirrorData::Dead
            }
        } else if let Some(revision) = dependency.strip_prefix("revision/") {
            if let Ok(revision) = revision.parse() {
                found_revision = Some(revision);
            } else {
                return MirrorData::Dead
            }
        }
    }
    if let (Some(release), Some(revision)) = (found_release, found_revision) {
        MirrorData::Alive {
            texlive_version: release,
            revision,
        }
    } else {
        MirrorData::Dead
    }
}

struct TlpdbByHash {
    mapping: std::sync::RwLock<HashMap<String, Arc<(Notify, watch::Sender<Option<MirrorData>>)>>>,
}
impl TlpdbByHash {
    pub fn new() -> Self {
        TlpdbByHash {
            mapping: std::sync::RwLock::new(HashMap::new()),
        }
    }
    pub async fn by_hash(&self, hash: &str, mirror: &str) -> MirrorData {
        let entry = {
            let mapping = self.mapping.read().unwrap();
            mapping.get(hash).cloned()
        };
        let entry = if let Some(entry) = entry {
            entry
        } else {
            let mut mapping = self.mapping.write().unwrap();
            if let Some(entry) = mapping.get(hash) {
                entry.clone()
            } else {
                let notify = Notify::new();
                notify.notify_one();
                mapping.insert(
                    hash.to_owned(),
                    Arc::new((notify, watch::Sender::new(None))),
                );
                mapping.get(hash).unwrap().clone()
            }
        };
        let mut receiver = entry.1.subscribe();
        if let Some(ref data) = *receiver.borrow_and_update() {
            return data.clone();
        }
        tokio::select! {
            _ = entry.0.notified() => {
                let Ok(tlpdb_content) = get_tlpdb(mirror).await else {
                    entry.0.notify_one();
                    return MirrorData::Dead
                };
                let mut digest = Sha512::new();
                digest.update(tlpdb_content.as_bytes());
                if hex::decode(hash).as_ref().map(|v| &v[..]) != Ok(&digest.finalize()) {
                    eprintln!("File checksum mismatch");
                    entry.0.notify_one();
                    return MirrorData::Dead
                }
                let data = get_mirror_data_from_tlpdb(&tlpdb_content);
                entry.1.send_replace(Some(data.clone()));
                data
            }
            res = receiver.changed() => {
                res.unwrap();
                receiver.borrow_and_update().as_ref().unwrap().clone()
            }
        }
    }
}

async fn get_tlpdb_hash(mirror: &str) -> Result<String, Error> {
    let tlpdb_url = format!("{}tlpkg/texlive.tlpdb.sha512", mirror);
    let response = reqwest::get(tlpdb_url).await?;
    if !response.status().is_success() {
        bail!("Retrieving tlpdb failed with status {}", response.status())
    }
    let mut response_text = response.text().await?;
    if response_text.len() <= 0x80 {
        bail!("Not a valid checksum file")
    }
    if &response_text[0x80..=0x80] != " " {
        bail!("Not a valid checksum file")
    }
    response_text.truncate(0x80);
    Ok(response_text)
}

async fn get_tlpdb(mirror: &str) -> Result<String, Error> {
    let tlpdb_url = format!("{}tlpkg/texlive.tlpdb.xz", mirror);
    let response = reqwest::get(tlpdb_url).await?;
    if !response.status().is_success() {
        bail!("Retrieving tlpdb failed with status {}", response.status())
    }
    let mut response_text = String::new();
    XzDecoder::new(response.bytes().await?.as_ref()).read_to_string(&mut response_text)?;
    Ok(response_text)
}

async fn process_mirrors(mirrors: Mirrors, exclusions: Arc<HashSet<Mirror>>) -> Result<MirrorsWithData, Error> {
    let mappings = Arc::new(TlpdbByHash::new());
    let mut continent_set: JoinSet<Result<(String, ContinentMirrorsWithData), Error>> =
        JoinSet::new();
    for (name, continent_mirrors) in mirrors.0 {
        let exclusions = exclusions.clone();
        let mappings = mappings.clone();
        continent_set.spawn(async {
            let exclusions = exclusions;
            let mappings = mappings;
            let mut country_set: JoinSet<Result<(String, CountryMirrorsWithData), Error>> =
                JoinSet::new();
            for (name, country_mirrors) in continent_mirrors.0 {
                let exclusions = exclusions.clone();
                let mappings = mappings.clone();
                country_set.spawn(async {
                    let exclusions = exclusions;
                    let mappings = mappings;
                    let mut mirror_set: JoinSet<(Mirror, MirrorData)> = JoinSet::new();
                    for mirror in country_mirrors.0 {
                        let exclusions = exclusions.clone();
                        let mappings = mappings.clone();
                        mirror_set.spawn(async {
                            let exclusions = exclusions;
                            let mappings = mappings;
                            let mirror = mirror;
                            let tl_mirror = Mirror(format!("{}systems/texlive/tlnet/", mirror.0));
                            if exclusions.contains(&mirror) {
                                return (tl_mirror, MirrorData::Excluded)
                            }
                            if let Ok(result) = timeout(Duration::from_secs(15), async {
                                let hash = get_tlpdb_hash(&tl_mirror.0).await;
                                if let Ok(hash) = hash {
                                    mappings.by_hash(&hash, &tl_mirror.0).await
                                } else {
                                    MirrorData::Dead
                                }
                            })
                            .await
                            {
                                (tl_mirror, result)
                            } else {
                                (tl_mirror, MirrorData::Timeout)
                            }
                        });
                    }
                    let mut result = HashMap::new();
                    while let Some(join_handle) = mirror_set.join_next().await {
                        let (key, value) = join_handle?;
                        result.insert(key, value);
                    }
                    Ok((name, CountryMirrorsWithData(result)))
                });
            }
            let mut result = HashMap::new();
            while let Some(join_handle) = country_set.join_next().await {
                let (key, value) = join_handle??;
                result.insert(key, value);
            }
            Ok((name, ContinentMirrorsWithData(result)))
        });
    }
    let mut result = HashMap::new();
    while let Some(join_handle) = continent_set.join_next().await {
        let (key, value) = join_handle??;
        result.insert(key, value);
    }
    Ok(MirrorsWithData(result))
}

enum Mode {
    Mirrors {
        exclusions: HashSet<Mirror>,
    },
    Schema,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mode = {
        let mut args = env::args();
        if let None = args.next() {
            bail!("program name argument missing")
        }
        match args.next().as_ref().map(|a| a.as_str()) {
            Some("mirrors") => {
                let mut exclusions = HashSet::new();
                while let Some(arg) = args.next() {
                    if &arg == "--exclude" {
                        let Some(mirror) = args.next() else { break };
                        exclusions.insert(Mirror(mirror));
                    }
                }
                Mode::Mirrors { exclusions }
            },
            Some("schema") => Mode::Schema,
            Some(_) => bail!("Unknown mode"),
            None => bail!("Pass `mirrors' or `schema' to specify mode"),
        }
    };

    match mode {
        Mode::Mirrors { exclusions } => {
            let mirrors = parse_mirrors().await?;
            let processed = process_mirrors(mirrors, Arc::new(exclusions)).await?;

            println!("{}", serde_json::to_string_pretty(&processed)?);
        }
        Mode::Schema => {
            println!("{}", serde_json::to_string_pretty(&schema_for!(MirrorsWithData))?);
        }
    }
    Ok(())
}
