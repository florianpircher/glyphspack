use anyhow::{bail, Result};
use clap::{Arg, Command};
use std::path::Path;

#[macro_use]
extern crate pest_derive;

mod pack;
mod plist;
mod unpack;

const ARG_KEY_OUTFILE: &str = "OUT";
const ARG_KEY_FILE: &str = "IN";
const ARG_KEY_FORCE: &str = "FORCE";
const ARG_KEY_QUIET: &str = "QUIET";

const FILE_EXT_STANDALONE: &str = "glyphs";
const FILE_EXT_PACKAGE: &str = "glyphspackage";
const FILE_EXT_GLYPH: &str = "glyph";

const FILE_PACKAGE_FONTINFO: &str = "fontinfo.plist";
const FILE_PACKAGE_ORDER: &str = "order.plist";
const FILE_PACKAGE_UI_STATE: &str = "UIState.plist";
const FILE_PACKAGE_GLYPHS: &str = "glyphs";

const KEY_DISPLAY_STRINGS_PACKAGE: &str = "displayStrings";
const KEY_DISPLAY_STRINGS_STANDALONE: &str = "DisplayStrings";
const KEY_GLYPH_NAME: &str = "glyphname";
const KEY_GLYPHS: &str = "glyphs";

enum Operation {
    Pack,
    Unpack,
}

fn main() -> Result<()> {
    let config = Command::new("glyphspack")
        .version("1.0")
        .author("Florian Pircher <florian@formkunft.com>")
        .about("Convert between .glyphs and .glyphspackage files. The conversion direction is automatically detected depending on whether <FILE> is a directory or not.")
        .after_help("See the Glyphs Handbook <https://glyphsapp.com/learn> for details on the standalone and the package format flavors.")
        .arg(
            Arg::new(ARG_KEY_OUTFILE)
                .short('o')
                .long("out")
                .help("The output file")
                .value_name("OUTFILE")
                .takes_value(true),
        )
        .arg(
            Arg::new(ARG_KEY_FORCE)
                .short('f')
                .long("force")
                .help("Overwrites output file if it already exists"),
        )
        .arg(
            Arg::new(ARG_KEY_QUIET)
                .short('q')
                .long("quiet")
                .help("Suppresses log messages"),
        )
        .arg(
            Arg::new(ARG_KEY_FILE)
                .help("The input file")
                .value_name("FILE")
                .required(true)
                .index(1),
        )
        .get_matches();
    let force = config.is_present(ARG_KEY_FORCE);
    let quiet = config.is_present(ARG_KEY_QUIET);
    let out_file = config.value_of(ARG_KEY_OUTFILE);
    let in_file = config.value_of(ARG_KEY_FILE).unwrap();
    let in_path = Path::new(in_file);

    if !in_path.exists() {
        bail!("<FILE> does not exist: {}", in_path.display());
    }

    let operation = if in_path.is_dir() {
        Operation::Unpack
    } else {
        Operation::Pack
    };

    let out_path = match out_file {
        Some(file) => Path::new(file).to_owned(),
        None => match operation {
            Operation::Pack => in_path.with_extension(FILE_EXT_PACKAGE),
            Operation::Unpack => in_path.with_extension(FILE_EXT_STANDALONE),
        },
    };

    if !force && out_path.exists() {
        bail!("<OUTFILE> already exists: {}", out_path.display());
    }

    match operation {
        Operation::Pack => {
            if !quiet {
                eprintln!("Packing {} into {}", in_path.display(), out_path.display());
            }
            pack::pack(in_path, &out_path, force)
        }
        Operation::Unpack => {
            if !quiet {
                eprintln!(
                    "Unpacking {} into {}",
                    in_path.display(),
                    out_path.display()
                );
            }
            unpack::unpack(in_path, &out_path)
        }
    }
}
