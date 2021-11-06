use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use regex::{Captures, Regex};
use std::fs;
use std::path::Path;

use crate::{
    plist, FILE_EXT_GLYPH, FILE_PACKAGE_FONTINFO, FILE_PACKAGE_GLYPHS, FILE_PACKAGE_ORDER,
    FILE_PACKAGE_UI_STATE, KEY_DISPLAY_STRINGS_PACKAGE, KEY_DISPLAY_STRINGS_STANDALONE, KEY_GLYPHS,
    KEY_GLYPH_NAME,
};

pub fn pack(in_path: &Path, out_path: &Path, force: bool) -> Result<()> {
    // Read Standalong File

    let standalone_code = fs::read_to_string(&in_path)
        .with_context(|| format!("cannot read font info at {}", in_path.display()))?;
    let standalone = plist::parse(plist::Root::Dict, &standalone_code)
        .with_context(|| format!("cannot parse font info at {}", in_path.display()))?;
    let standalone = match standalone.value {
        plist::Value::Dict(x) => x,
        _ => unreachable!(),
    };

    let mut fontinfo: Vec<&str> = Vec::new();
    let mut order: Vec<&str> = Vec::new();
    let mut ui_state: Vec<String> = Vec::new();
    let mut glyphs: Vec<(&str, &str)> = Vec::new();

    for (key, slice, code) in standalone {
        match key {
            KEY_DISPLAY_STRINGS_STANDALONE => {
                let code = format!("{} = (\n{}\n);", KEY_DISPLAY_STRINGS_PACKAGE, slice.code);
                ui_state.push(code);
            }
            KEY_GLYPHS => {
                let glyph_slices = match slice.value {
                    plist::Value::Array(items) => items,
                    _ => bail!("non-array `{}` in {}", KEY_GLYPHS, in_path.display()),
                };
                for glyph_slice in glyph_slices {
                    let glyph = match glyph_slice.value {
                        plist::Value::Dict(pairs) => pairs,
                        _ => bail!("non-dict glyph in {}", in_path.display()),
                    };
                    let mut glyph_name: Option<&str> = None;
                    for (key, slice, _) in glyph {
                        if key == KEY_GLYPH_NAME {
                            match slice.value {
                                plist::Value::String(name) => glyph_name = Some(name),
                                _ => bail!(
                                    "non-string `{}` in {}",
                                    KEY_GLYPH_NAME,
                                    in_path.display()
                                ),
                            };
                        }
                    }
                    let glyph_name = glyph_name.with_context(|| {
                        format!(
                            "missing `{}` in glyph in {}",
                            KEY_GLYPH_NAME,
                            in_path.display()
                        )
                    })?;

                    order.push(glyph_name);
                    glyphs.push((glyph_name, glyph_slice.code));
                }
            }
            _ => {
                fontinfo.push(code);
            }
        }
    }

    // Create Directories

    if force && out_path.is_dir() {
        fs::remove_dir_all(&out_path).with_context(|| {
            format!(
                "cannot overwrite existing directory at {}",
                out_path.display()
            )
        })?;
    }

    fs::create_dir_all(&out_path)
        .with_context(|| format!("cannot create package at {}", out_path.display()))?;

    let glyphs_path = out_path.join(FILE_PACKAGE_GLYPHS);
    fs::create_dir(&glyphs_path).with_context(|| {
        format!(
            "cannot create glyphs diractory at {}",
            glyphs_path.display()
        )
    })?;

    // Write Font Info

    let fontinfo_path = out_path.join(FILE_PACKAGE_FONTINFO);
    plist::write_dict_file(&fontinfo_path, &fontinfo)?;

    // Write Order

    let order_path = out_path.join(FILE_PACKAGE_ORDER);
    plist::write_array_file(&order_path, &order)?;

    // Write UI State

    if !ui_state.is_empty() {
        let ui_state_path = out_path.join(FILE_PACKAGE_UI_STATE);
        plist::write_dict_file(&ui_state_path, &ui_state.iter().map(|x| &**x).collect())?;
    }

    // Write Glyphs

    let init_dot_regex = Regex::new(r"^\.").unwrap();
    let capital_regex = Regex::new(r"[A-Z]").unwrap();

    glyphs.into_par_iter().try_for_each(|(glyphname, code)| {
        let file_stem = glyphname;
        let file_stem = init_dot_regex.replacen(file_stem, 1, "_");
        let file_stem = capital_regex.replace_all(file_stem.as_ref(), |captures: &Captures| {
            format!(
                "{}_",
                captures.get(0).unwrap().as_str().to_ascii_uppercase()
            )
        });

        let glyph_path = glyphs_path.join(format!("{}.{}", file_stem, FILE_EXT_GLYPH));
        plist::write_dict_file(&glyph_path, &vec![code])?;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
