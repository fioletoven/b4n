use nom::bytes::complete::{take_while, take_while1};
use nom::character::complete::char;
use nom::{IResult, Parser};
use std::str::FromStr;

use crate::logs::formats::parse_quoted;
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
            "level" => level = Some(LogLevel::from_str(value).ok()?),
            "msg" => message = Some(value.to_owned()),
            "t" | "ts" | "time" | "timestamp" => {},
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

fn parse_field(input: &str) -> IResult<&str, (&str, &str)> {
    let (rest, key) = take_while1(|ch: char| !ch.is_whitespace() && !ch.is_control() && ch != '=' && ch != '"').parse(input)?;
    let (rest, _) = char('=').parse(rest)?;
    let (rest, value) = parse_value(rest)?;

    Ok((rest, (key, value)))
}

fn parse_value(input: &str) -> IResult<&str, &str> {
    if input.starts_with('"') {
        parse_quoted(input)
    } else {
        take_while(|ch: char| !ch.is_whitespace() && !ch.is_control() && ch != '"').parse(input)
    }
}
