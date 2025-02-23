use nom::{
    branch::alt,
    bytes::complete::{is_a, tag},
    character::complete,
    combinator::{eof, map, map_opt, recognize},
    error::ParseError,
    multi::{many0, many0_count, many1_count, many_till},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Parser,
};
use std::collections::HashMap;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BorrowedContactEntry<'a> {
    pub announce: Option<&'a str>,
    pub bugs: Option<&'a str>,
    pub development: Option<&'a str>,
    pub home: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub support: Option<&'a str>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BorrowedCatalogueEntry<'a> {
    pub name: Option<&'a str>,
    pub alias: Option<&'a str>,
    pub also: Option<&'a str>,
    pub contact: BorrowedContactEntry<'a>,
    pub ctan: Option<&'a str>,
    pub license: Option<&'a str>,
    pub topics: Option<&'a str>,
    pub version: Option<&'a str>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BorrowedFiles<'a> {
    pub size: u32,
    pub files: Vec<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BorrowedEntry<'a> {
    pub name: &'a str,
    pub catalogue: BorrowedCatalogueEntry<'a>,
    pub category: Category,
    pub container_checksum: Option<&'a str>,
    pub container_size: Option<&'a str>,
    pub depend: Vec<&'a str>,
    pub execute: Vec<&'a str>,
    pub long_desc: Option<String>,
    pub post_action: Vec<&'a str>,
    pub relocated: bool,
    pub revision: u32,
    pub short_desc: Option<&'a str>,
    pub bin_files: HashMap<&'a str, BorrowedFiles<'a>>,
    pub doc_container_checksum: Option<&'a str>,
    pub doc_container_size: Option<&'a str>,
    pub doc_files: Option<BorrowedFiles<'a>>,
    pub run_files: Option<BorrowedFiles<'a>>,
    pub src_container_checksum: Option<&'a str>,
    pub src_container_size: Option<&'a str>,
    pub src_files: Option<BorrowedFiles<'a>>,
}

enum Field<'a> {
    Catalogue(&'a str),
    CatalogueAlias(&'a str),
    CatalogueAlso(&'a str),
    CatalogueContactAnnounce(&'a str),
    CatalogueContactBugs(&'a str),
    CatalogueContactDevelopment(&'a str),
    CatalogueContactHome(&'a str),
    CatalogueContactRepository(&'a str),
    CatalogueContactSupport(&'a str),
    CatalogueCtan(&'a str),
    CatalogueLicense(&'a str),
    CatalogueTopics(&'a str),
    CatalogueVersion(&'a str),
    Category(Category),
    ContainerChecksum(&'a str),
    ContainerSize(&'a str),
    Depend(&'a str),
    Execute(&'a str),
    LongDesc(&'a str),
    PostAction(&'a str),
    Relocated,
    Revision(u32),
    ShortDesc(&'a str),
    BinFiles(&'a str, BorrowedFiles<'a>),
    DocContainerChecksum(&'a str),
    DocContainerSize(&'a str),
    DocFiles(BorrowedFiles<'a>),
    RunFiles(BorrowedFiles<'a>),
    SrcContainerChecksum(&'a str),
    SrcContainerSize(&'a str),
    SrcFiles(BorrowedFiles<'a>),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Category {
    TLCore,
    Scheme,
    Collection,
    ConTeXt,
    Package,
}

fn comment<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, (), E> {
    map(
        many0_count(tuple((
            tag("#"),
            complete::not_line_ending,
            complete::line_ending,
        ))),
        |_| (),
    )
}

fn linebreak<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, (), E> {
    map(pair(complete::line_ending, comment()), |_| ())
}

fn parse_name<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, &'a str, E> {
    delimited(
        tag("name "),
        complete::not_line_ending,
        pair(tag("\n"), comment()),
    )
}

fn parse_category<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("category "),
        map(
            alt((
                map(tag("TLCore"), |_| Category::TLCore),
                map(tag("Scheme"), |_| Category::Scheme),
                map(tag("Collection"), |_| Category::Collection),
                map(tag("ConTeXt"), |_| Category::ConTeXt),
                map(tag("Package"), |_| Category::Package),
            )),
            |v| Field::Category(v),
        ),
        linebreak(),
    )
}

fn parse_containersize<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("containersize "),
        map(complete::not_line_ending, |v| Field::ContainerSize(v)),
        linebreak(),
    )
}

fn parse_containerchecksum<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("containerchecksum "),
        map(complete::not_line_ending, |v| Field::ContainerChecksum(v)),
        linebreak(),
    )
}

fn parse_doccontainersize<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("doccontainersize "),
        map(complete::not_line_ending, |v| Field::DocContainerSize(v)),
        linebreak(),
    )
}

fn parse_doccontainerchecksum<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("doccontainerchecksum "),
        map(complete::not_line_ending, |v| {
            Field::DocContainerChecksum(v)
        }),
        linebreak(),
    )
}

fn parse_srccontainersize<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("srccontainersize "),
        map(complete::not_line_ending, |v| Field::SrcContainerSize(v)),
        linebreak(),
    )
}

fn parse_srccontainerchecksum<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("srccontainerchecksum "),
        map(complete::not_line_ending, |v| {
            Field::SrcContainerChecksum(v)
        }),
        linebreak(),
    )
}

fn parse_catalogue<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue "),
        map(complete::not_line_ending, |v| Field::Catalogue(v)),
        linebreak(),
    )
}

fn parse_catalogue_ctan<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-ctan "),
        map(complete::not_line_ending, |v| Field::CatalogueCtan(v)),
        linebreak(),
    )
}

fn parse_catalogue_topics<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-topics "),
        map(complete::not_line_ending, |v| Field::CatalogueTopics(v)),
        linebreak(),
    )
}

fn parse_catalogue_version<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-version "),
        map(complete::not_line_ending, |v| Field::CatalogueVersion(v)),
        linebreak(),
    )
}

fn parse_catalogue_license<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-license "),
        map(complete::not_line_ending, |v| Field::CatalogueLicense(v)),
        linebreak(),
    )
}

fn parse_catalogue_also<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-also "),
        map(complete::not_line_ending, |v| Field::CatalogueAlso(v)),
        linebreak(),
    )
}

fn parse_catalogue_alias<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-alias "),
        map(complete::not_line_ending, |v| Field::CatalogueAlias(v)),
        linebreak(),
    )
}

fn parse_catalogue_contact_announce<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-contact-announce "),
        map(complete::not_line_ending, |v| {
            Field::CatalogueContactAnnounce(v)
        }),
        linebreak(),
    )
}

fn parse_catalogue_contact_development<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-contact-development "),
        map(complete::not_line_ending, |v| {
            Field::CatalogueContactDevelopment(v)
        }),
        linebreak(),
    )
}

fn parse_catalogue_contact_repository<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-contact-repository "),
        map(complete::not_line_ending, |v| {
            Field::CatalogueContactRepository(v)
        }),
        linebreak(),
    )
}

fn parse_catalogue_contact_bugs<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E>
{
    delimited(
        tag("catalogue-contact-bugs "),
        map(complete::not_line_ending, |v| {
            Field::CatalogueContactBugs(v)
        }),
        linebreak(),
    )
}

fn parse_catalogue_contact_home<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E>
{
    delimited(
        tag("catalogue-contact-home "),
        map(complete::not_line_ending, |v| {
            Field::CatalogueContactHome(v)
        }),
        linebreak(),
    )
}

fn parse_execute<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("execute "),
        map(complete::not_line_ending, |v| Field::Execute(v)),
        linebreak(),
    )
}

fn parse_postaction<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("postaction "),
        map(complete::not_line_ending, |v| Field::PostAction(v)),
        linebreak(),
    )
}

fn parse_catalogue_contact_support<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("catalogue-contact-support "),
        map(complete::not_line_ending, |v| {
            Field::CatalogueContactSupport(v)
        }),
        linebreak(),
    )
}

fn parse_relocated<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    terminated(map(tag("relocated 1"), |_| Field::Relocated), linebreak())
}

fn parse_revision<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("revision "),
        map(complete::u32, |v| Field::Revision(v)),
        linebreak(),
    )
}

fn parse_shortdesc<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("shortdesc "),
        map(complete::not_line_ending, |v| Field::ShortDesc(v)),
        linebreak(),
    )
}

fn parse_longdesc<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("longdesc "),
        map(complete::not_line_ending, |v| Field::LongDesc(v)),
        linebreak(),
    )
}

fn parse_depend<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    delimited(
        tag("depend "),
        map(complete::not_line_ending, |v| Field::Depend(v)),
        linebreak(),
    )
}

fn parse_files<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, BorrowedFiles<'a>, E> {
    map(
        separated_pair(
            preceded(tag("size="), complete::u32),
            linebreak(),
            many0(delimited(tag(" "), complete::not_line_ending, linebreak())),
        ),
        |(size, files)| BorrowedFiles { size, files },
    )
}

fn parse_runfiles<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    preceded(tag("runfiles "), map(parse_files(), |v| Field::RunFiles(v)))
}

fn parse_binfiles<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    preceded(
        tag("binfiles "),
        map(
            pair(
                delimited(
                    tag("arch="),
                    recognize(many1_count(alt((complete::alphanumeric1, is_a("_-"))))),
                    tag(" "),
                ),
                parse_files(),
            ),
            |(arch, v)| Field::BinFiles(arch, v),
        ),
    )
}

fn parse_srcfiles<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    preceded(tag("srcfiles "), map(parse_files(), |v| Field::SrcFiles(v)))
}

fn parse_docfiles<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    preceded(tag("docfiles "), map(parse_files(), |v| Field::DocFiles(v)))
}

fn parse_field<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Field<'a>, E> {
    // Split only because nom does not support such big alt tuples
    alt((
        alt((
            parse_category(),
            parse_catalogue(),
            parse_catalogue_ctan(),
            parse_catalogue_topics(),
            parse_catalogue_version(),
            parse_catalogue_license(),
            parse_catalogue_also(),
            parse_catalogue_alias(),
            parse_catalogue_contact_announce(),
            parse_catalogue_contact_development(),
            parse_catalogue_contact_repository(),
            parse_catalogue_contact_bugs(),
            parse_catalogue_contact_home(),
            parse_catalogue_contact_support(),
            parse_execute(),
            parse_postaction(),
            parse_relocated(),
            parse_revision(),
            parse_shortdesc(),
            parse_longdesc(),
            parse_depend(),
        )),
        alt((
            parse_runfiles(),
            parse_binfiles(),
            parse_docfiles(),
            parse_srcfiles(),
            parse_containersize(),
            parse_containerchecksum(),
            parse_doccontainersize(),
            parse_doccontainerchecksum(),
            parse_srccontainersize(),
            parse_srccontainerchecksum(),
        )),
    ))
}

pub fn parse_entry<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, BorrowedEntry<'a>, E> {
    map_opt(
        pair(
            parse_name(),
            many_till(parse_field(), alt((linebreak(), map(eof, |_| ())))),
        ),
        |(name, (fields, _))| {
            let mut catalogue = None;
            let mut catalogue_alias = None;
            let mut catalogue_also = None;
            let mut catalogue_contact_announce = None;
            let mut catalogue_contact_bugs = None;
            let mut catalogue_contact_development = None;
            let mut catalogue_contact_home = None;
            let mut catalogue_contact_repository = None;
            let mut catalogue_contact_support = None;
            let mut catalogue_ctan = None;
            let mut catalogue_license = None;
            let mut catalogue_topics = None;
            let mut catalogue_version = None;
            let mut category = None;
            let mut container_checksum = None;
            let mut container_size = None;
            let mut depend = Vec::new();
            let mut execute = Vec::new();
            let mut long_desc: Option<String> = None;
            let mut post_action = Vec::new();
            let mut relocated = false;
            let mut revision = None;
            let mut short_desc = None;
            let mut bin_files = HashMap::new();
            let mut doc_container_checksum = None;
            let mut doc_container_size = None;
            let mut doc_files = None;
            let mut run_files = None;
            let mut src_container_checksum = None;
            let mut src_container_size = None;
            let mut src_files = None;

            for field in fields {
                match field {
                    Field::Catalogue(value) => {
                        if let Some(_) = catalogue {
                            return None;
                        }
                        catalogue = Some(value);
                    }
                    Field::CatalogueAlias(value) => {
                        if let Some(_) = catalogue_alias {
                            return None;
                        }
                        catalogue_alias = Some(value);
                    }
                    Field::CatalogueAlso(value) => {
                        if let Some(_) = catalogue_also {
                            return None;
                        }
                        catalogue_also = Some(value);
                    }
                    Field::CatalogueContactAnnounce(value) => {
                        if let Some(_) = catalogue_contact_announce {
                            return None;
                        }
                        catalogue_contact_announce = Some(value);
                    }
                    Field::CatalogueContactBugs(value) => {
                        if let Some(_) = catalogue_contact_bugs {
                            return None;
                        }
                        catalogue_contact_bugs = Some(value);
                    }
                    Field::CatalogueContactDevelopment(value) => {
                        if let Some(_) = catalogue_contact_development {
                            return None;
                        }
                        catalogue_contact_development = Some(value);
                    }
                    Field::CatalogueContactHome(value) => {
                        if let Some(_) = catalogue_contact_home {
                            return None;
                        }
                        catalogue_contact_home = Some(value);
                    }
                    Field::CatalogueContactRepository(value) => {
                        if let Some(_) = catalogue_contact_repository {
                            return None;
                        }
                        catalogue_contact_repository = Some(value);
                    }
                    Field::CatalogueContactSupport(value) => {
                        if let Some(_) = catalogue_contact_support {
                            return None;
                        }
                        catalogue_contact_support = Some(value);
                    }
                    Field::CatalogueCtan(value) => {
                        if let Some(_) = catalogue_ctan {
                            return None;
                        }
                        catalogue_ctan = Some(value);
                    }
                    Field::CatalogueLicense(value) => {
                        if let Some(_) = catalogue_license {
                            return None;
                        }
                        catalogue_license = Some(value);
                    }
                    Field::CatalogueTopics(value) => {
                        if let Some(_) = catalogue_topics {
                            return None;
                        }
                        catalogue_topics = Some(value);
                    }
                    Field::CatalogueVersion(value) => {
                        if let Some(_) = catalogue_version {
                            return None;
                        }
                        catalogue_version = Some(value);
                    }
                    Field::Category(value) => {
                        if let Some(_) = category {
                            return None;
                        }
                        category = Some(value);
                    }
                    Field::ContainerChecksum(value) => {
                        if let Some(_) = container_checksum {
                            return None;
                        }
                        container_checksum = Some(value);
                    }
                    Field::ContainerSize(value) => {
                        if let Some(_) = container_size {
                            return None;
                        }
                        container_size = Some(value);
                    }
                    Field::Depend(value) => {
                        depend.push(value);
                    }
                    Field::Execute(value) => {
                        execute.push(value);
                    }
                    Field::LongDesc(value) => {
                        if let Some(ref mut existing) = long_desc {
                            *existing += " ";
                            *existing += value;
                        } else {
                            long_desc = Some(value.to_owned());
                        }
                    }
                    Field::PostAction(value) => {
                        post_action.push(value);
                    }
                    Field::Relocated => {
                        relocated = true;
                    }
                    Field::Revision(value) => {
                        if let Some(_) = revision {
                            return None;
                        }
                        revision = Some(value);
                    }
                    Field::ShortDesc(value) => {
                        if let Some(_) = short_desc {
                            return None;
                        }
                        short_desc = Some(value);
                    }
                    Field::BinFiles(arch, value) => {
                        bin_files.insert(arch, value);
                    }
                    Field::DocContainerChecksum(value) => {
                        if let Some(_) = doc_container_checksum {
                            return None;
                        }
                        doc_container_checksum = Some(value);
                    }
                    Field::DocContainerSize(value) => {
                        if let Some(_) = doc_container_size {
                            return None;
                        }
                        doc_container_size = Some(value);
                    }
                    Field::DocFiles(value) => {
                        if let Some(_) = doc_files {
                            return None;
                        }
                        doc_files = Some(value);
                    }
                    Field::RunFiles(value) => {
                        if let Some(_) = run_files {
                            return None;
                        }
                        run_files = Some(value);
                    }
                    Field::SrcContainerChecksum(value) => {
                        if let Some(_) = src_container_checksum {
                            return None;
                        }
                        src_container_checksum = Some(value);
                    }
                    Field::SrcContainerSize(value) => {
                        if let Some(_) = src_container_size {
                            return None;
                        }
                        src_container_size = Some(value);
                    }
                    Field::SrcFiles(value) => {
                        if let Some(_) = src_files {
                            return None;
                        }
                        src_files = Some(value);
                    }
                }
            }
            Some(BorrowedEntry {
                name,
                catalogue: BorrowedCatalogueEntry {
                    name: catalogue,
                    alias: catalogue_alias,
                    also: catalogue_also,
                    contact: BorrowedContactEntry {
                        announce: catalogue_contact_announce,
                        bugs: catalogue_contact_bugs,
                        development: catalogue_contact_development,
                        home: catalogue_contact_home,
                        repository: catalogue_contact_repository,
                        support: catalogue_contact_support,
                    },
                    ctan: catalogue_ctan,
                    license: catalogue_license,
                    topics: catalogue_topics,
                    version: catalogue_version,
                },
                category: category?,
                container_checksum,
                container_size,
                depend,
                execute,
                long_desc,
                post_action,
                relocated,
                revision: revision?,
                short_desc,
                bin_files,
                doc_container_checksum,
                doc_container_size,
                doc_files,
                run_files,
                src_container_checksum,
                src_container_size,
                src_files,
            })
        },
    )
}

pub fn parse_entries<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Vec<BorrowedEntry<'a>>, E>
{
    many0(parse_entry())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_entry() {
        let input = "name 00texlive.config
category TLCore
revision 54074
shortdesc TeX Live network archive option settings
longdesc This package contains configuration options for the TeX Live
longdesc archive. If container_split_{doc,src}_files/1 are dependencies,
longdesc the {doc,src} files are split into separate containers
longdesc (.tar.xz) during container build time. This has NO effect on
longdesc the appearance within texlive.tlpdb. It is only on the
longdesc container level. The container_format/WHATEVER specifies the
longdesc format, currently \"xz\" is the only supported value (generating
longdesc .tar.xz files). release/YYYY specifies the release number as
longdesc used in the installer. minrelease/ZZZZ specifies the minimum
longdesc release for which this repository is valid, i.e., a release of
longdesc ZZZZ or later can theoretically be upgraded. Further
longdesc information concerning upgrades can be found at
longdesc http://www.tug.org/texlive/upgrade.html frozen/[01] specifies
longdesc whether the release has been frozen The default values are
longdesc taken from TeXLive::TLConfig::TLPDBConfigs hash values at tlpdb
longdesc creation time but can be overridden here if necessary. For
longdesc information on the 00texlive prefix see
longdesc 00texlive.installation(.tlpsrc)
depend container_format/xz
depend container_split_doc_files/1
depend container_split_src_files/1
depend frozen/0
depend minrelease/2016
depend release/2024
depend revision/70660
";
        let (remaining, parsed) = parse_entry::<nom::error::VerboseError<_>>()
            .parse(&input)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing string literal: {remaining}")
        }
        assert_eq!(
            parsed,
            BorrowedEntry {
                name: "00texlive.config",
                category: Category::TLCore,
                catalogue: BorrowedCatalogueEntry {
                    name: None,
                    alias: None,
                    also: None,
                    contact: BorrowedContactEntry {
                        announce: None,
                        bugs: None,
                        development: None,
                        home: None,
                        repository: None,
                        support: None,
                    },
                    ctan: None,
                    license: None,
                    topics: None,
                    version: None,
                },
                container_checksum: None,
                container_size: None,
                depend: vec![
                    "container_format/xz",
                    "container_split_doc_files/1",
                    "container_split_src_files/1",
                    "frozen/0",
                    "minrelease/2016",
                    "release/2024",
                    "revision/70660",
                ],
                execute: Vec::new(),
                long_desc: Some("This package contains configuration options for the TeX Live archive. If container_split_{doc,src}_files/1 are dependencies, the {doc,src} files are split into separate containers (.tar.xz) during container build time. This has NO effect on the appearance within texlive.tlpdb. It is only on the container level. The container_format/WHATEVER specifies the format, currently \"xz\" is the only supported value (generating .tar.xz files). release/YYYY specifies the release number as used in the installer. minrelease/ZZZZ specifies the minimum release for which this repository is valid, i.e., a release of ZZZZ or later can theoretically be upgraded. Further information concerning upgrades can be found at http://www.tug.org/texlive/upgrade.html frozen/[01] specifies whether the release has been frozen The default values are taken from TeXLive::TLConfig::TLPDBConfigs hash values at tlpdb creation time but can be overridden here if necessary. For information on the 00texlive prefix see 00texlive.installation(.tlpsrc)".to_owned()),
                post_action: Vec::new(),
                relocated: false,
                revision: 54074,
                short_desc: Some("TeX Live network archive option settings"),
                bin_files: HashMap::new(),
                doc_container_checksum: None,
                doc_container_size: None,
                doc_files: None,
                run_files: None,
                src_container_checksum: None,
                src_container_size: None,
                src_files: None,
            }
        );
    }
    #[test]
    fn test_parse_full_file() {
        let content =
            std::fs::read_to_string("/CTAN/systems/texlive/tlnet/tlpkg/texlive.tlpdb").unwrap();
        let (remaining, parsed) = parse_entries::<nom::error::VerboseError<_>>()
            .parse(&content)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing file: {remaining}")
        }
        println!("Parsed: {parsed:#?}");
        use std::collections::HashSet;
        let mut collected = HashSet::new();
        for entry in parsed {
            collected.insert(entry.catalogue.contact.announce);
        }
        println!("collected: {collected:#?}");
    }
}
const _IGNORED: &'static str = "
#[derive(Debug, PartialEq, Eq)]
pub struct BorrowedEntry<'a> {
    pub catalogue_license: Option<&'a str>, -- enum?
    pub catalogue_topics: Option<&'a str>, -- list, enum?
    pub catalogue_version: Option<&'a str>,
    pub category: Category,
    pub container_checksum: Option<&'a str>,
    pub container_size: Option<&'a str>,
    pub depend: Vec<&'a str>,
    pub execute: Vec<&'a str>,
    pub long_desc: Option<String>,
    pub post_action: Vec<&'a str>,
    pub relocated: bool,
    pub revision: u32,
    pub short_desc: Option<&'a str>,
    pub bin_files: Vec<Vec<&'a str>>,
    pub doc_container_checksum: Option<&'a str>,
    pub doc_container_size: Option<&'a str>,
    pub doc_files: Option<Vec<&'a str>>,
    pub run_files: Option<Vec<&'a str>>,
    pub src_container_checksum: Option<&'a str>,
    pub src_container_size: Option<&'a str>,
    pub src_files: Option<Vec<&'a str>>,
}
";
