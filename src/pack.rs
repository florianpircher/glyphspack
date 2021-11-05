use anyhow::Result;
use std::path::Path;

pub fn pack(in_path: &Path, out_path: &Path) -> Result<()> {
    eprintln!("Packing {} into {}.", in_path.display(), out_path.display());
    Ok(())
}
