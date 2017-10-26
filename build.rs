extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate tl_codegen;


use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;


mod error {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SetLogger(::log::SetLoggerError);
        }
    }
}


const TL_SCHEMA_DIR:       &'static str = "./tl";
const TL_SCHEMA_LIST_FILE: &'static str = "./tl/tl-schema-list.txt";
const RUST_SCHEMA_FILE:    &'static str = "./src/schema.rs";

fn collect_input() -> error::Result<String> {
    let mut tl_files = BufReader::new(File::open(TL_SCHEMA_LIST_FILE)?).lines().filter_map(|line| {
        match line {
            Ok(ref line) if line.starts_with("//") => None,  // This line is a comment
            Ok(filename) => Some(Ok(Path::new(TL_SCHEMA_DIR).join(filename))),
            Err(e) => Some(Err(e)),  // Do not ignore errors
        }
    }).collect::<io::Result<Vec<PathBuf>>>()?;

    tl_files.sort();
    debug!("Files detected: {:?}", &tl_files);
    println!("cargo:rerun-if-changed={}", TL_SCHEMA_LIST_FILE);

    let mut input = String::new();
    for tl_file in tl_files {
        File::open(&tl_file)?.read_to_string(&mut input)?;
        println!("cargo:rerun-if-changed={}", tl_file.to_string_lossy());
    }

    Ok(input)
}

fn run() -> error::Result<()> {
    env_logger::init()?;

    let input = collect_input()?;
    let code = tl_codegen::generate_code_for(&input);
    debug!("Code size: {} bytes", code.as_str().len());

    File::create(RUST_SCHEMA_FILE)?.write_all(code.as_str().as_bytes())?;
    debug!("Successful write to {}", RUST_SCHEMA_FILE);

    Command::new("rustfmt")
        .arg("--write-mode")
        .arg("overwrite")
        .arg(RUST_SCHEMA_FILE)
        .status()?;

    Ok(())
}

quick_main!(run);
