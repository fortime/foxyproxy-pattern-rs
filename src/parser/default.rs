use std::io::{BufRead, BufReader, Read};

use anyhow::Result;

use crate::foxyproxy::{Pattern, Rule};

pub struct DefaultParser;

impl DefaultParser {
    pub fn parse<R>(&self, src: R) -> Result<Vec<Rule>>
    where
        R: Read,
    {
        let mut res = vec![];
        for line in BufReader::new(src).lines() {
            let line = line?;
            let mut line = line.as_str();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            if line.starts_with('/')
                || line
                    .chars()
                    .all(|c| c.is_ascii_digit() || ".:|@".contains(c))
            {
                eprintln!("unsupported pattern: {}", line);
                continue;
            }
            let include = if let Some(l) = line.strip_prefix("@@") {
                line = l;
                false
            } else {
                true
            };
            let pattern = if let Some(line) = line.strip_prefix("||") {
                match_or_wildcard(line)
            } else if let Some(line) = line.strip_prefix("|") {
                wildcard(line)
            } else if line.starts_with(".") {
                wildcard(&format!("*.{}", line))
            } else {
                match_or_wildcard(line)
            };
            res.push(Rule::new(
                format!("Include Pattern[{}/{}]", line, include),
                pattern,
                true,
                include,
            ));
        }
        Ok(res)
    }
}

fn match_or_wildcard(s: &str) -> Pattern {
    if s.contains('*') {
        wildcard(s)
    } else {
        Pattern::Match(format!("*://*.{}/*", s))
    }
}

fn wildcard(s: &str) -> Pattern {
    let s = if let Some(s) = s.strip_prefix("http://") {
        s
    } else if let Some(s) = s.strip_prefix("https://") {
        s
    } else if let Some(s) = s.strip_prefix("*://") {
        s
    } else {
        s
    };
    if s.contains('/') {
        Pattern::Wildcard(format!("*://{}", s))
    } else {
        Pattern::Wildcard(format!("*://{}/", s))
    }
}
