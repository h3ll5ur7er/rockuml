//! Embeds `assets/` (stdlib, themes, fonts...) into the binary as a sorted table of path → contents.
//!
//! Files are deflated here and inflated on first use, which keeps the binary small; those in formats that are
//! compressed already are embedded as they are. The `stdlib` and `emoji` features decide whether those
//! folders are embedded at all.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::DeflateEncoder;

const ALREADY_COMPRESSED: [&str; 3] = ["br", "spm", "png"];

/// Folders that are embedded only with the feature of the same name.
const OPTIONAL_FOLDERS: [&str; 2] = ["stdlib", "emoji"];

fn main() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
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
        .filter(|(name, _)| is_enabled(name))
        .collect();
    entries.sort();

    let deflated_directory = out_dir.join("deflated");
    // An array rather than a slice, as the assets inflate in place.
    let mut table = format!("static FILES: [Asset; {}] = [\n", entries.len());
    for (name, file) in entries {
        let is_compressed = file
            .extension()
            .is_some_and(|extension| ALREADY_COMPRESSED.contains(&&*extension.to_string_lossy()));
        let (constructor, embedded) = if is_compressed {
            ("stored", file)
        } else {
            let deflated = deflated_directory.join(&name);
            fs::create_dir_all(deflated.parent().unwrap()).unwrap();
            fs::write(&deflated, deflate(&fs::read(&file).unwrap())).unwrap();
            ("deflated", deflated)
        };
        writeln!(
            table,
            "    Asset::{constructor}({name:?}, include_bytes!({:?})),",
            embedded.display().to_string()
        )
        .unwrap();
    }
    table.push_str("];\n");
    fs::write(out_dir.join("assets.rs"), table).unwrap();
}

fn is_enabled(name: &str) -> bool {
    let folder = name.split('/').next().unwrap();
    !OPTIONAL_FOLDERS.contains(&folder)
        || env::var_os(format!("CARGO_FEATURE_{}", folder.to_uppercase())).is_some()
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
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
