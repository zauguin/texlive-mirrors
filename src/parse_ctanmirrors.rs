use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use nom::{
    bytes::complete::{is_not, tag},
    character::complete::multispace0,
    combinator::map,
    error::ParseError,
    multi::many0,
    sequence::{delimited, pair, separated_pair, terminated},
    Parser,
};

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct BorrowedMirror<'a>(pub &'a str);
#[derive(Debug, PartialEq, Eq)]
pub struct BorrowedCountryMirrors<'a>(pub HashSet<BorrowedMirror<'a>>);
#[derive(Debug, PartialEq, Eq)]
pub struct BorrowedContinentMirrors<'a>(pub HashMap<&'a str, BorrowedCountryMirrors<'a>>);
#[derive(Debug, PartialEq, Eq)]
pub struct BorrowedMirrors<'a>(pub HashMap<&'a str, BorrowedContinentMirrors<'a>>);

#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Mirror(pub String);
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct CountryMirrorList(pub HashSet<Mirror>);
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct ContinentMirrorList(pub HashMap<String, CountryMirrorList>);
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct MirrorList(pub HashMap<String, ContinentMirrorList>);

impl<'a> From<BorrowedMirror<'a>> for Mirror {
    fn from(BorrowedMirror(value): BorrowedMirror<'a>) -> Self {
        Self(value.to_owned())
    }
}
impl<'a> From<BorrowedCountryMirrors<'a>> for CountryMirrorList {
    fn from(BorrowedCountryMirrors(value): BorrowedCountryMirrors<'a>) -> Self {
        Self(FromIterator::from_iter(value.into_iter().map(From::from)))
    }
}
impl<'a> From<BorrowedContinentMirrors<'a>> for ContinentMirrorList {
    fn from(BorrowedContinentMirrors(value): BorrowedContinentMirrors<'a>) -> Self {
        Self(FromIterator::from_iter(
            value
                .into_iter()
                .map(|(k, v)| (k.to_owned(), From::from(v))),
        ))
    }
}
impl<'a> From<BorrowedMirrors<'a>> for MirrorList {
    fn from(BorrowedMirrors(value): BorrowedMirrors<'a>) -> Self {
        Self(FromIterator::from_iter(
            value
                .into_iter()
                .map(|(k, v)| (k.to_owned(), From::from(v))),
        ))
    }
}

// from https://github.com/rust-bakery/nom/blob/main/doc/nom_recipes.md
/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F, O, E: ParseError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

fn parse_string_literal<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, &'a str, E> {
    delimited(tag("'"), is_not("'"), tag("'"))
}

fn parse_mirror_entry<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, BorrowedMirror<'a>, E> {
    terminated(
        map(parse_string_literal(), |url| BorrowedMirror(url)),
        pair(ws(tag("=>")), tag("1")),
    )
}

fn parse_mirror_set<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, BorrowedCountryMirrors<'a>, E> {
    delimited(
        tag("{"),
        map(
            many0(ws(terminated(parse_mirror_entry(), tag(",")))),
            |mirrors| BorrowedCountryMirrors(HashSet::from_iter(mirrors.into_iter())),
        ),
        tag("}"),
    )
}

fn parse_country<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, (&'a str, BorrowedCountryMirrors<'a>), E> {
    separated_pair(
        parse_string_literal(),
        ws(tag("=>")),
        ws(parse_mirror_set()),
    )
}

fn parse_continent<'a, E: ParseError<&'a str>>(
) -> impl Parser<&'a str, (&'a str, BorrowedContinentMirrors<'a>), E> {
    separated_pair(
        parse_string_literal(),
        ws(tag("=>")),
        map(
            delimited(
                tag("{"),
                many0(ws(terminated(parse_country(), tag(",")))),
                tag("}"),
            ),
            |mirrors| BorrowedContinentMirrors(HashMap::from_iter(mirrors.into_iter())),
        ),
    )
}

pub fn parse_mirrors<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, BorrowedMirrors<'a>, E> {
    delimited(
        tag("$mirrors = {"),
        map(
            many0(ws(terminated(parse_continent(), tag(",")))),
            |mirrors| BorrowedMirrors(HashMap::from_iter(mirrors.into_iter())),
        ),
        tag("};\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_string_interval() {
        let input = "'https://example.com/pub/tex/'";
        let (remaining, parsed) = parse_string_literal::<nom::error::VerboseError<_>>()
            .parse(&input)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing string literal: {remaining}")
        }
        assert_eq!(parsed, "https://example.com/pub/tex/");
    }

    #[test]
    fn test_parse_mirror_set() {
        let input = "{
            'https://example.com/pub/tex/' => 1,
            'foobar' => 1,
        }";
        let (remaining, parsed) = parse_mirror_set::<nom::error::VerboseError<_>>()
            .parse(&input)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing mirror set: {remaining}")
        }
        assert_eq!(
            parsed,
            BorrowedCountryMirrors(
                [
                    BorrowedMirror("https://example.com/pub/tex/"),
                    BorrowedMirror("foobar")
                ]
                .into()
            )
        );
    }

    #[test]
    fn test_parse_country() {
        let input = "'Germany' => {
            'https://example.com/pub/tex/' => 1,
            'foobar' => 1,
        }";
        let (remaining, parsed) = parse_country::<nom::error::VerboseError<_>>()
            .parse(&input)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing mirror set: {remaining}")
        }
        assert_eq!(
            parsed,
            (
                "Germany",
                BorrowedCountryMirrors(
                    [
                        BorrowedMirror("https://example.com/pub/tex/"),
                        BorrowedMirror("foobar")
                    ]
                    .into()
                )
            )
        );
    }

    #[test]
    fn test_parse_continent() {
        let input = "'Europe' => {
            'Germany' => {
                'https://example.com/pub/tex/' => 1,
                'foo' => 1,
            },
            'Spain' => {
                'https://example.org/pub/tex/' => 1,
                'bar' => 1,
            },
        }";
        let (remaining, parsed) = parse_continent::<nom::error::VerboseError<_>>()
            .parse(&input)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing mirror set: {remaining}")
        }
        assert_eq!(
            parsed,
            (
                "Europe",
                BorrowedContinentMirrors(
                    [
                        (
                            "Germany",
                            BorrowedCountryMirrors(
                                [
                                    BorrowedMirror("https://example.com/pub/tex/"),
                                    BorrowedMirror("foo")
                                ]
                                .into()
                            )
                        ),
                        (
                            "Spain",
                            BorrowedCountryMirrors(
                                [
                                    BorrowedMirror("https://example.org/pub/tex/"),
                                    BorrowedMirror("bar")
                                ]
                                .into()
                            )
                        )
                    ]
                    .into()
                )
            )
        );
    }

    #[test]
    fn test_parse_mirrors() {
        let input = "$mirrors = {
            'Europe' => {
                'Germany' => {
                    'https://example.com/pub/tex/' => 1,
                    'foo' => 1,
                },
                'Spain' => {
                    'https://example.org/pub/tex/' => 1,
                    'bar' => 1,
                },
            },
        };\n";
        let (remaining, parsed) = parse_mirrors::<nom::error::VerboseError<_>>()
            .parse(&input)
            .unwrap();
        if !remaining.is_empty() {
            panic!("Unexpected remaining tokens after parsing mirror set: {remaining}")
        }
        assert_eq!(
            parsed,
            BorrowedMirrors(
                [(
                    "Europe",
                    BorrowedContinentMirrors(
                        [
                            (
                                "Germany",
                                BorrowedCountryMirrors(
                                    [
                                        BorrowedMirror("https://example.com/pub/tex/"),
                                        BorrowedMirror("foo")
                                    ]
                                    .into()
                                )
                            ),
                            (
                                "Spain",
                                BorrowedCountryMirrors(
                                    [
                                        BorrowedMirror("https://example.org/pub/tex/"),
                                        BorrowedMirror("bar")
                                    ]
                                    .into()
                                )
                            )
                        ]
                        .into()
                    )
                )]
                .into()
            )
        );
    }
}
