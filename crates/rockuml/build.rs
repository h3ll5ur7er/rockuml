//! Embeds `assets/` (stdlib, themes) into the binary as a sorted table of path → bytes.

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    println!("cargo::rerun-if-changed={}", assets.display());

    let mut files = Vec::new();
    collect(&assets, &mut files);
    let mut entries: Vec<(String, PathBuf)> = files
        .into_iter()
        .map(|file| {
            let name = file
                .strip_prefix(&assets)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (name, file)
        })
        .collect();
    entries.sort();

    let mut table = String::from("pub(crate) static FILES: &[(&str, &[u8])] = &[\n");
    for (name, file) in entries {
        writeln!(
            table,
            "    ({name:?}, include_bytes!({:?})),",
            file.display().to_string()
        )
        .unwrap();
    }
    table.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("assets.rs"),
        table,
    )
    .unwrap();
}

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}
