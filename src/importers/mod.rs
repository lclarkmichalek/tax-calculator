use anyhow::Result;
use log::{warn};
use std::{
    ffi::OsStr,
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
};

use manifest::Manifest;

pub mod manifest;
pub mod vanguard;

pub(crate) fn find_imports(imports_directory: &Path) -> Result<Vec<(Manifest, PathBuf)>> {
    let mut imports = vec![];

    for dir_entry in read_dir(imports_directory)? {
        let dir_entry = dir_entry?;
        if dir_entry.metadata()?.is_dir() {
            continue;
        }
        if dir_entry.path().extension() != Some(OsStr::new("toml")) {
            continue;
        }

        let manifest_contents = read_to_string(&dir_entry.path())?;
        let manifest: Manifest = toml::from_str(&manifest_contents)?;

        let import_path = dir_entry
            .path()
            .with_extension(manifest.platform.file_extension());

        if !import_path.exists() {
            warn!(
                "{:?} exists but {:?} does not. is provider correct?",
                dir_entry.path(),
                import_path
            );
        }
        imports.push((manifest, import_path))
    }
    Ok(imports)
}
