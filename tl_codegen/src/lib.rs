#[macro_use]
extern crate error_chain;
extern crate pom;
#[cfg(feature = "printing")]
#[macro_use]
extern crate quote;
extern crate syn;
#[cfg(feature = "parsing")]
extern crate synom;


mod ast;
mod error;
mod generator;
mod parser;


pub use generator::generate_items_for;

#[cfg(feature = "printing")]
pub use generator::generate_code_for;
