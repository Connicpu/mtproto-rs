extern crate tl_codegen;

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Command;


const TL_SCHEMA_DIR: &'static str = "./tl";
const RUST_SCHEMA_FILE: &'static str = "./src/schema.rs";

fn collect_input() -> io::Result<String> {
    let mut tl_files = fs::read_dir(TL_SCHEMA_DIR)?.filter_map(|dir_entry| {
        match dir_entry {
            Ok(entry) => {
                let path = entry.path();

                if entry.path().extension().and_then(OsStr::to_str) == Some("tl") {
                    Some(Ok(path))
                } else {
                    None
                }
            },
            Err(e) => Some(Err(e)),
        }
    }).collect::<io::Result<Vec<PathBuf>>>()?;

    tl_files.sort();

    let mut input = String::new();
    for tl_file in tl_files {
        File::open(&tl_file)?.read_to_string(&mut input)?;
        println!("cargo:rerun-if-changed={}", tl_file.to_string_lossy());
    }

    Ok(input)
}

fn run() -> io::Result<()> {
    let input = collect_input()?;
    let code = tl_codegen::generate_code_for(&input);

    File::create(RUST_SCHEMA_FILE)?.write_all(code.as_str().as_bytes())?;

    Command::new("rustfmt")
        .arg("--write-mode")
        .arg("overwrite")
        .arg(RUST_SCHEMA_FILE)
        .status()?;

    Ok(())
}

fn main() {
    run().unwrap();
}
