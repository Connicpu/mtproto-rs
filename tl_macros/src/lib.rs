extern crate syntex;
extern crate syntex_syntax;

use syntex::Registry;

use syntex_syntax::ast;
use syntex_syntax::codemap::Span;
use syntex_syntax::ext::base::{ExtCtxt, MacEager, MacResult};
use syntex_syntax::ext::build::AstBuilder;
use syntex_syntax::parse::token::InternedString;

//pub fn expand_tl_complex

pub fn register(registry: &mut Registry) {
    //registry.add_macro("tl_complex", expand_tl_complex);
}
