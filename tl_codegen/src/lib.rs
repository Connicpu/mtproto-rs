#[macro_use]
extern crate error_chain;
extern crate petgraph;
extern crate pom;
#[cfg(feature = "printing")]
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

#[cfg(feature = "printing")]
pub use generator::generate_code_for;
