extern crate tl_codegen;

use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    println!("{}", tl_codegen::generate_code_for(&input));
}
