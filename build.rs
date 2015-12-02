extern crate syntex;
extern crate tl_macros;

use std::env;
use std::path::Path;

fn main() {
    let mut registry = syntex::Registry::new();
    tl_macros::register(&mut registry);
    
    let src = Path::new("src/tl/complex_types/mod.in.rs");
    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("tl/complex_types/mod.rs");
    
    //registry.expand("tl_complex", &src, &dst).unwrap();
}

