use nom::branch::alt;
use nom::bytes::complete::{take_while_m_n, take_while1};
use nom::character::complete::{char, digit1, space1};
use nom::combinator::{map, opt, recognize, value};
use nom::{IResult, Parser};

use crate::{LogLevel, ParsedLogLine};

#[cfg(test)]
#[path = "./klog.tests.rs"]
mod content_tests;

/// Parses a raw klog-formatted line into a `ParsedLogLine`.
pub fn parse(line: &str) -> Option<ParsedLogLine> {
    let (rest, header) = parse_header(line).ok()?;

    let (message, fields) = parse_message_and_fields(rest);
    let context = build_context(header.file, header.line_no, header.thread_id, &fields);

    Some(ParsedLogLine {
        level: header.level,
        message: message.to_owned(),
        context,
    })
}

fn build_context(file: &str, line_no: &str, thread_id: &str, fields: &[&str]) -> Option<String> {
    let capacity = fields.iter().filter(|f| !f.is_empty()).map(|f| 2 + f.len()).sum::<usize>();
    let capacity = if !file.is_empty() {
        file.len() + line_no.len() + 3 + capacity
    } else {
        capacity
    };
    let capacity = if !thread_id.is_empty() {
        "tid=".len() + thread_id.len() + 2 + capacity
    } else {
        capacity
    };

    if capacity == 0 {
        return None;
    }

    let mut out = String::with_capacity(capacity);

    if !file.is_empty() {
        out.push_str(file);
        out.push(':');
        out.push_str(line_no);
    }

    if !thread_id.is_empty() {
        if !out.is_empty() {
            out.push_str(", ");
        }

        out.push_str("tid=");
        out.push_str(thread_id);
    }

    for field in fields.iter().filter(|f| !f.is_empty()) {
        if !out.is_empty() {
            out.push_str(", ");
        }

        out.push_str(field);
    }

    Some(out)
}

struct ParsedHeader<'a> {
    level: LogLevel,
    file: &'a str,
    line_no: &'a str,
    thread_id: &'a str,
}

/// Parses header of the message: `Lmmdd hh:mm:ss.uuuuuu threadid file:line] `
fn parse_header(input: &str) -> IResult<&str, ParsedHeader<'_>> {
    map(
        (
            parse_level,
            digits_n(4), // mmdd
            space1,
            parse_timestamp, // hh:mm:ss.uuuuuu
            space1,
            digit1, // thread ID
            space1,
            parse_source,
            opt(char(' ')),
        ),
        |(level, _, _, _, _, tid, _, (file, line), _)| ParsedHeader {
            level,
            file,
            line_no: line,
            thread_id: tid,
        },
    )
    .parse(input)
}

/// Parses klog's single-letter severity marker.
fn parse_level(input: &str) -> IResult<&str, LogLevel> {
    alt((
        value(LogLevel::Info, char('I')),
        value(LogLevel::Warn, char('W')),
        value(LogLevel::Error, char('E')),
        value(LogLevel::Fatal, char('F')),
    ))
    .parse(input)
}

/// Parses `hh:mm:ss.uuuuuu`.
fn parse_timestamp(input: &str) -> IResult<&str, &str> {
    recognize((digits_n(2), char(':'), digits_n(2), char(':'), digits_n(2), char('.'), digit1)).parse(input)
}

/// Parses `file:line]`.
fn parse_source(input: &str) -> IResult<&str, (&str, &str)> {
    map(
        (take_while1(|c: char| c != ':' && c != ']'), char(':'), digit1, char(']')),
        |(file, _, line, _)| (file, line),
    )
    .parse(input)
}

/// Parses exactly `n` ASCII digits.
fn digits_n(n: usize) -> impl FnMut(&str) -> IResult<&str, &str> {
    move |input: &str| take_while_m_n(n, n, |c: char| c.is_ascii_digit()).parse(input)
}

/// Splits klog's structured tail into the quoted message and the `key=value` fields that follow it.
fn parse_message_and_fields(rest: &str) -> (&str, Vec<&str>) {
    let rest = rest.trim_start();

    let (message, remainder) = if let Some(stripped) = rest.strip_prefix('"') {
        match find_closing_quote(stripped) {
            Some(end) => (&stripped[..end], &stripped[end + 1..]),
            None => (stripped, ""),
        }
    } else {
        (rest, "")
    };

    let fields = parse_fields(remainder.trim_start());
    (message, fields)
}

/// Parses `key=value key2="v2" key3=[a b]` string into a vector of `key=value` tokens.
fn parse_fields(input: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut rest = input;

    while !rest.is_empty() {
        let Some(eq_pos) = rest.find('=') else { break };
        let value_len = find_value_end(&rest[eq_pos + 1..]);
        let field_end = eq_pos + 1 + value_len;

        let field = rest[..field_end].trim();
        if !field.is_empty() {
            fields.push(field);
        }

        rest = rest[field_end..].trim_start();
    }

    fields
}

/// Returns the end index of the value starting at `s`.
fn find_value_end(s: &str) -> usize {
    if let Some(stripped) = s.strip_prefix('"') {
        return match find_closing_quote(stripped) {
            Some(end) => end + 2, // opening quote + content + closing quote
            None => s.len(),
        };
    }

    let mut stack: Vec<char> = Vec::new();
    for (idx, ch) in s.char_indices() {
        match ch {
            '[' => stack.push(']'),
            '{' => stack.push('}'),
            c @ (']' | '}') if stack.last() == Some(&c) => {
                stack.pop();
            },
            c if c.is_whitespace() && stack.is_empty() => return idx,
            _ => {},
        }
    }

    s.len()
}

fn find_closing_quote(s: &str) -> Option<usize> {
    let mut escaped = false;
    for (idx, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some(idx),
            _ => {},
        }
    }

    None
}
