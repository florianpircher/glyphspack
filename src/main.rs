use anyhow::{bail, Result};
use clap::{App, Arg};
use std::path::Path;

#[macro_use]
extern crate pest_derive;

mod pack;
mod plist;
mod unpack;

enum Operation {
    Pack,
    Unpack,
}

fn main() -> Result<()> {
    let config = App::new("glyphspack")
        .version("0.2")
        .author("Florian Pircher <florian@addpixel.net>")
        .about("Convert between .glyphs and .glyphspackage files.")
        .arg(
            Arg::with_name("OUT")
                .short("o")
                .long("out")
                .help("The output file")
                .value_name("OUTFILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FORCE")
                .short("f")
                .long("force")
                .help("Overwrite output file if it already exists"),
        )
        .arg(
            Arg::with_name("IN")
                .help("The input file")
                .value_name("FILE")
                .required(true)
                .index(1),
        )
        .get_matches();
    let force = config.is_present("FORCE");
    let out_file = config.value_of("OUT");
    let in_file = config.value_of("IN").unwrap();
    let in_path = Path::new(in_file);

    if !in_path.exists() {
        bail!("FILE does not exist {}", in_path.display());
    }

    let operation = if in_path.is_dir() {
        Operation::Unpack
    } else {
        Operation::Pack
    };

    let out_path = match out_file {
        Some(file) => Path::new(file).to_owned(),
        None => match operation {
            Operation::Pack => in_path.with_extension("glyphspackage"),
            Operation::Unpack => in_path.with_extension("glyphs"),
        },
    };

    if !force && out_path.exists() {
        bail!("OUTFILE already exists {}", out_path.display());
    }

    match operation {
        Operation::Pack => pack::pack(in_path, &out_path),
        Operation::Unpack => unpack::unpack(in_path, &out_path),
    }
}
