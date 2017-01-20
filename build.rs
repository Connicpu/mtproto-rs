extern crate syntex;
extern crate tl_macros;

use std::env;
use std::path::Path;

fn compile(input: &str, out: &str) {
    let mut registry = syntex::Registry::new();
    tl_macros::register(&mut registry);

    let src = Path::new(input);
    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join(out);
    registry.expand("", &src, &dst).unwrap();
}

fn main() {
    compile("src/tl/complex_types/mod.in.rs", "complex_types.rs");
    compile("src/rpc/functions/mod.in.rs", "rpc_functions.rs");
}

