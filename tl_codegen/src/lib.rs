#[macro_use]
extern crate error_chain;
extern crate petgraph;
extern crate pom;
#[macro_use]
extern crate quote;
extern crate syn;
#[cfg(feature = "parsing")]
extern crate synom;


mod analyzer;
mod ast;
mod error;
mod generator;
mod parser;


pub use generator::generate_ast_for;
pub use generator::generate_code_for;
