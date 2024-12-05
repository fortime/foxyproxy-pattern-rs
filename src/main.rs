use std::{
    fs,
    io::{self, Read, Write}, process,
};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use foxyproxy::Rule;

mod encoding;
mod foxyproxy;
mod parser;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ParserType {
    #[default]
    Default,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Encoding {
    #[default]
    Raw,
    Base64,
    Hex,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Source of patterns to be parse, it can be a file/url.
    #[arg(short, long, value_name = "FILE/URL")]
    src: String,

    /// Encoding of the source.
    #[arg(long, value_enum, default_value_t)]
    src_encoding: Encoding,

    /// Parser of the source.
    #[arg(short, long, value_enum, default_value_t)]
    parser: ParserType,

    /// Destination file for saving the foxyproxy patterns.
    #[arg(short, long, value_name = "FILE")]
    dst: String,

    /// Encoding of the destination.
    #[arg(long, value_enum, default_value_t)]
    dst_encoding: Encoding,
}

fn src_stream(src: &str, encoding: Encoding) -> Result<Box<dyn Read>> {
    #[cfg(feature = "reqwest")]
    {
        let lowercase_src = src.to_lowercase();
        if ["http://", "https://"]
            .into_iter()
            .any(|s| lowercase_src.starts_with(s))
        {
            return Ok(encoding::decode(
                reqwest::blocking::get(src)?.error_for_status()?,
                encoding,
            ));
        }
    }

    if src == "-" {
        Ok(encoding::decode(io::stdin(), encoding))
    } else {
        Ok(encoding::decode(fs::File::open(src)?, encoding))
    }
}

fn dst_stream(dst: &str, encoding: Encoding) -> Result<Box<dyn Write>> {
    if dst == "-" {
        Ok(encoding::encode(io::stdout(), encoding))
    } else {
        Ok(encoding::encode(fs::File::create(dst)?, encoding))
    }
}

fn run(args: Args) -> Result<()> {
    let mut src_stream = src_stream(&args.src, args.src_encoding)?;

    let rules: Vec<Rule> = match args.parser {
        ParserType::Default => parser::DefaultParser.parse(&mut src_stream)?,
    };

    let mut dst_stream = dst_stream(&args.dst, args.dst_encoding)?;

    serde_json::to_writer_pretty(&mut dst_stream, &rules)?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("run command failed: {e}");
        process::exit(1);
    }
}
