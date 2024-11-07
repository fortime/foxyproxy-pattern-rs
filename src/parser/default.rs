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
            if line.is_empty() || line.starts_with("!") {
                continue;
            }
            let include = if line.starts_with("@@") {
                line = &line[2..];
                false
            } else {
                true
            };
            let pattern = if line.starts_with("||") {
                format!("*://*.{}", &line[2..])
            } else if line.starts_with("|") {
                line[1..].to_string()
            } else if line.starts_with(".") {
                // exclude parent
                let parent_domain = &line[1..];
                res.push(Rule::new(
                    format!("Exclude Parent Domain[{}]", parent_domain),
                    Pattern::Wildcard(format!("*://{}", parent_domain)),
                    true,
                    !include,
                ));
                format!("*://*{}", line)
            } else {
                format!("*://*.{}", line)
            };
            res.push(Rule::new(
                format!("Include Pattern[{}]", pattern),
                Pattern::Wildcard(pattern),
                true,
                include,
            ));
        }
        Ok(res)
    }
}
