use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use crate::plist;
use crate::{
    FILE_EXT_GLYPH, FILE_PACKAGE_FONTINFO, FILE_PACKAGE_GLYPHS, FILE_PACKAGE_ORDER,
    FILE_PACKAGE_UI_STATE, KEY_DISPLAY_STRINGS_PACKAGE, KEY_DISPLAY_STRINGS_STANDALONE, KEY_GLYPHS,
    KEY_GLYPH_NAME,
};

pub fn unpack(in_path: &Path, out_path: &Path) -> Result<()> {
    // Read Font Info

    let fontinfo_path = in_path.join(FILE_PACKAGE_FONTINFO);
    let fontinfo_code = fs::read_to_string(&fontinfo_path)
        .with_context(|| format!("cannot read font info at {}", fontinfo_path.display()))?;
    let fontinfo = plist::parse(plist::Root::Dict, &fontinfo_code)
        .with_context(|| format!("cannot parse font info at {}", fontinfo_path.display()))?;
    let fontinfo = match fontinfo.value {
        plist::Value::Dict(x) => x,
        _ => unreachable!(),
    };
    let mut file_contents: Vec<(String, String)> = fontinfo
        .into_iter()
        .map(|(key, _, code)| (key.to_string(), code.to_string()))
        .collect();

    // Read Order

    let order_path = in_path.join(FILE_PACKAGE_ORDER);
    let order_code = fs::read_to_string(&order_path)
        .with_context(|| format!("cannot read order at {}", order_path.display()))?;
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

    let glyphs_path = in_path.join(FILE_PACKAGE_GLYPHS);
    let glyph_paths_iter = fs::read_dir(&glyphs_path)
        .with_context(|| format!("cannot read glyph listing at {}", glyphs_path.display()));
    let glyph_paths: Vec<_> = glyph_paths_iter
        .unwrap()
        .flat_map(|dir_entry| {
            let dir_entry = dir_entry
                .with_context(|| format!("cannot read glyph entry at {}", glyphs_path.display()))
                .unwrap();
            if dir_entry.path().extension() == Some(OsStr::new(FILE_EXT_GLYPH)) {
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
                .with_context(|| format!("cannot read glyph at {}", path.display()))
                .unwrap();
            let glyphs_dict = plist::parse(plist::Root::Dict, &glyph_code)
                .with_context(|| format!("cannot parse glyph at {}", path.display()))
                .unwrap();
            let pairs = match glyphs_dict.value {
                plist::Value::Dict(x) => x,
                _ => unreachable!(),
            };

            for (key, slice, _) in pairs.into_iter() {
                if key == KEY_GLYPH_NAME {
                    let glyphname = match slice.value {
                        plist::Value::String(x) => x,
                        _ => {
                            panic!(
                                "non-string `{}` value for glyph at {}",
                                KEY_GLYPH_NAME,
                                path.display()
                            );
                        }
                    };
                    return (glyphname.to_string(), glyph_code);
                }
            }

            panic!(
                "missing `{}` in glyph at {}",
                KEY_GLYPH_NAME,
                path.display()
            );
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
    let glyphs_code = format!("{} = (\n{}\n);", KEY_GLYPHS, glyphs_code_value);
    file_contents.push((KEY_GLYPHS.to_string(), glyphs_code));

    // Read UI State

    let ui_state_path = in_path.join(FILE_PACKAGE_UI_STATE);
    if let Ok(ui_state_code) = fs::read_to_string(&ui_state_path) {
        let ui_state = plist::parse(plist::Root::Dict, &ui_state_code)
            .with_context(|| format!("cannot parse UI state at {}", ui_state_path.display()))?;
        let ui_state = match ui_state.value {
            plist::Value::Dict(pairs) => pairs,
            _ => unreachable!(),
        };

        for (key, slice, code) in ui_state {
            let (key, code) = match key {
                KEY_DISPLAY_STRINGS_PACKAGE => (
                    KEY_DISPLAY_STRINGS_STANDALONE,
                    format!("{} = (\n{}\n);", KEY_DISPLAY_STRINGS_STANDALONE, slice.code),
                ),
                _ => (key, code.to_string()),
            };

            file_contents.push((key.to_string(), code));
        }
    }

    // Write Standalone Glyphs File

    file_contents.sort_by(|(a, _), (b, _)| a.cmp(b));
    plist::write_dict_file(
        &out_path,
        &file_contents.iter().map(|(_, x)| &**x).collect(),
    )?;

    Ok(())
}
