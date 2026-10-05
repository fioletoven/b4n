use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, take_while, take_while1};
use nom::character::complete::char;
use nom::combinator::{map, opt, value};
use nom::sequence::delimited;
use nom::{IResult, Parser};
use std::borrow::Cow;
use std::str::FromStr;

use crate::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./logfmt.tests.rs"]
mod content_tests;

/// Parses a logfmt line containing a recognized `level` and a `msg` field.
pub fn parse(line: &str) -> Option<ParsedLogLine> {
    let mut rest = line.trim();
    let mut level = None;
    let mut message = None;
    let mut fields = Vec::new();

    while !rest.is_empty() {
        let (remainder, (key, value)) = parse_field(rest).ok()?;
        match key {
            "level" => level = Some(LogLevel::from_str(&value).ok()?),
            "msg" => message = Some(value.into_owned()),
            "ts" | "time" | "timestamp" => {},
            _ => fields.push(&rest[..rest.len() - remainder.len()]),
        }

        if !remainder.is_empty() && !remainder.starts_with(char::is_whitespace) {
            return None;
        }

        rest = remainder.trim_start();
    }

    Some(ParsedLogLine {
        level: level?,
        message: message?,
        context: if fields.is_empty() { None } else { Some(fields.join(", ")) },
    })
}

fn parse_field(input: &str) -> IResult<&str, (&str, Cow<'_, str>)> {
    let (rest, key) = take_while1(|ch: char| !ch.is_whitespace() && !ch.is_control() && ch != '=' && ch != '"').parse(input)?;
    let (rest, _) = char('=').parse(rest)?;
    let (rest, value) = parse_value(rest)?;

    Ok((rest, (key, value)))
}

fn parse_value(input: &str) -> IResult<&str, Cow<'_, str>> {
    if input.starts_with('"') {
        map(parse_quoted_value, Cow::Owned).parse(input)
    } else {
        map(
            take_while(|ch: char| !ch.is_whitespace() && !ch.is_control() && ch != '"'),
            Cow::Borrowed,
        )
        .parse(input)
    }
}

fn parse_quoted_value(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        map(
            opt(escaped_transform(
                take_while1(|ch: char| ch != '\\' && ch != '"' && !ch.is_control()),
                '\\',
                parse_escape,
            )),
            Option::unwrap_or_default,
        ),
        char('"'),
    )
    .parse(input)
}

fn parse_escape(input: &str) -> IResult<&str, &str> {
    alt((
        value("\n", char('n')),
        value("\r", char('r')),
        value("\t", char('t')),
        value("\\", char('\\')),
        value("\"", char('"')),
    ))
    .parse(input)
}
