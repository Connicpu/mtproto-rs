extern crate tl_codegen;

use std::io::{self, Read, Write};
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;

const TL_DIR: &str = "tl";
const OUTPUT_FILE: &str = "src/schema.rs";

fn main_result() -> Result<(), io::Error> {
    let mut files = fs::read_dir(TL_DIR)?
        .map(|r| r.map(|d| d.path()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    files.sort();

    let mut input = String::new();
    for file in files {
        File::open(&file)?.read_to_string(&mut input)?;
        println!("cargo:rerun-if-changed={}", file.to_string_lossy());
    }

    let code = tl_codegen::generate_code_for(&input);
    File::create(OUTPUT_FILE)?.write_all(code.as_bytes())?;
    Command::new("rustfmt")
        .arg("--write-mode")
        .arg("overwrite")
        .arg("src/schema.rs")
        .status()?;

    Ok(())
}

fn main() {
    main_result().unwrap();
}
