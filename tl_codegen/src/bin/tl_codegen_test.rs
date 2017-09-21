extern crate tl_codegen;

use std::io::Read;


fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    println!("{}", generate_from_input(&input));
}

#[cfg(feature = "printing")]
fn generate_from_input(input: &str) -> String {
    tl_codegen::generate_code_for(input).into_string()
}

#[cfg(not(feature = "printing"))]
fn generate_from_input(input: &str) -> String {
    format!("{:?}", tl_codegen::generate_items_for(input))
}
