#![feature(read_exact, associated_consts)]
extern crate byteorder;

use tl::parsing::{Schema, ReadContext, WriteContext};

pub mod tl;

pub fn standard_schema() -> Schema {
    let mut schema = Schema::new();
    
    schema.add_constructor(tl::Bool(true));
    schema.add_constructor(tl::Bool(false));
    
    schema
}

