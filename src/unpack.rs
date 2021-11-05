use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::plist;

pub fn unpack(in_path: &Path, out_path: &Path) -> Result<()> {
    eprintln!(
        "Unpacking {} into {}.",
        in_path.display(),
        out_path.display()
    );

    // Read Font Info

    let fontinfo_path = in_path.join("fontinfo.plist");
    let fontinfo_code = fs::read_to_string(&fontinfo_path)
        .with_context(|| format!("cannot read fontinfo.plist at {}", fontinfo_path.display()))?;
    let fontinfo = plist::parse(plist::Root::Dict, &fontinfo_code)
        .with_context(|| format!("cannot parse fontinfo at {}", fontinfo_path.display()))?;
    let fontinfo = match fontinfo.value {
        plist::Value::Dict(x) => x,
        _ => unreachable!(),
    };
    let mut file_contents: Vec<(String, String)> = fontinfo
        .into_iter()
        .map(|(key, _, code)| (key.to_string(), code.to_string()))
        .collect();

    // Read Order

    let order_path = in_path.join("order.plist");
    let order_code = fs::read_to_string(&order_path)
        .with_context(|| format!("cannot read order.plist at {}", order_path.display()))?;
    let order = plist::parse(plist::Root::Array, &order_code)
        .with_context(|| format!("cannot parse order at {}", order_path.display()))?;
    let order = match order.value {
        plist::Value::Array(x) => x
            .into_iter()
            .map(|entry| match entry.value {
                plist::Value::String(glyphname) => glyphname,
                _ => {
                    panic!("non-string glyph name in order at {}", order_path.display());
                }
            })
            .collect::<Vec<&str>>(),
        _ => unreachable!(),
    };

    // Read Glyphs

    let glyphs_path = in_path.join("glyphs");
    let glyph_paths_iter = fs::read_dir(&glyphs_path).with_context(|| {
        format!(
            "cannot read contents of glyphs directory at {}",
            glyphs_path.display()
        )
    });
    let glyph_paths: Vec<_> = glyph_paths_iter
        .unwrap()
        .flat_map(|dir_entry| {
            let dir_entry = dir_entry
                .with_context(|| {
                    format!(
                        "failed to read entry of glyphs directory at {}",
                        glyphs_path.display()
                    )
                })
                .unwrap();
            if dir_entry.path().extension() == Some(OsStr::new("glyph")) {
                return Some(dir_entry.path());
            } else {
                return None;
            }
        })
        .collect();
    let glyphs: HashMap<String, String> = glyph_paths
        .par_iter()
        .map(|path| {
            let glyph_code = fs::read_to_string(path)
                .with_context(|| format!("cannot read contents of glyph at {}", path.display()))
                .unwrap();
            let glyphs_dict = plist::parse(plist::Root::Dict, &glyph_code)
                .with_context(|| format!("cannot parse glyph at {}", path.display()))
                .unwrap();
            let pairs = match glyphs_dict.value {
                plist::Value::Dict(x) => x,
                _ => {
                    panic!("non-dictionary root value for glyph at {}", path.display());
                }
            };

            for (key, slice, _) in pairs.into_iter() {
                if key == "glyphname" {
                    let glyphname = match slice.value {
                        plist::Value::String(x) => x,
                        _ => {
                            panic!("non-string glyphname value for glyph at {}", path.display());
                        }
                    };
                    return (glyphname.to_string(), glyph_code);
                }
            }

            panic!("missing glyphname in glyph at {}", path.display());
        })
        .collect();
    let glyphs_code_value = order
        .iter()
        .map(|&glyphname| match glyphs.get(glyphname) {
            Some(glyph_code) => glyph_code.trim().to_string(),
            None => {
                panic!(
                    "missing glyph /{}; glyph appears in {} but not in {}",
                    glyphname,
                    order_path.display(),
                    glyphs_path.display()
                );
            }
        })
        .collect::<Vec<String>>()
        .join(",");

    // Read UI State

    let ui_state_path = in_path.join("UIState.plist");
    if let Ok(ui_state_code) = fs::read_to_string(&ui_state_path) {
        let ui_state = plist::parse(plist::Root::Dict, &ui_state_code)
            .with_context(|| format!("cannot read UI state at {}", ui_state_path.display()))?;
        let ui_state = match ui_state.value {
            plist::Value::Dict(pairs) => pairs,
            _ => unreachable!(),
        };
        let ui_state_codes: Vec<(&str, String)> = ui_state
            .into_iter()
            .map(|(key, slice, code)| {
                if key == "displayStrings" {
                    (
                        "DisplayStrings",
                        format!("DisplayStrings = (\n{}\n);", slice.code),
                    )
                } else {
                    (key, code.to_string())
                }
            })
            .collect();

        for (key, code) in ui_state_codes {
            file_contents.push((key.to_string(), code));
        }
    }

    // Write Glyphs File

    let glyphs_code = format!("glyphs = (\n{}\n);", glyphs_code_value);
    file_contents.push(("glyphs".to_string(), glyphs_code));
    file_contents.sort_by(|(a, _), (b, _)| a.cmp(b));

    let mut glyphs_file = fs::File::create(&out_path)
        .with_context(|| format!("cannot create glyphs file at {}", out_path.display()))?;

    write!(glyphs_file, "{{\n").unwrap();

    for (_, code) in file_contents {
        write!(glyphs_file, "{}\n", code).unwrap();
    }

    write!(glyphs_file, "}}\n").unwrap();

    Ok(())
}
