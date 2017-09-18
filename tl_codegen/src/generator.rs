use std::collections::BTreeMap;

use quote;

use ast::{Constructor, Delimiter, Item, Type, TypeFixupMap};
use parser;


pub fn generate_code_for(input: &str) -> quote::Tokens {
    let constructors = {
        let mut items = parser::parse_string(input).unwrap();
        filter_items(&mut items);
        partition_by_delimiter_and_namespace(items)
    };

    let layer = constructors.layer as i32;
    let items = vec![quote! {
        pub const LAYER: i32 = #layer;
    }];

    let variants_to_outputs: TypeFixupMap = constructors.types.iter()
        .flat_map(|(namespaces, constructor_map)| {
            constructor_map.iter().flat_map(move |(output, constructors)| {
                constructors.0.iter().filter_map(move |constructor| {
                    let variant_name = match constructor.variant {
                        Type::Named(ref n) => n,
                        _ => return None,
                    };

                    let mut full_output: Vec<String> = namespaces.iter().cloned().collect();
                    full_output.push(output.clone());

                    Some((variant_name.clone(), full_output))
                })
            })
        })
        .collect();

    quote! {
        #(#items)*
    }
}

fn filter_items(items: &mut Vec<Item>) {
    items.retain(|item| {
        let c = match *item {
            Item::Constructor(ref c) => c,
            _ => return true,
        };

        // Blacklist some annoying inconsistencies.
        match c.variant.name() {
            Some("true") |
            Some("vector") => false,
            _ => true,
        }
    });
}

fn partition_by_delimiter_and_namespace(items: Vec<Item>) -> AllConstructors {
    let mut current = Delimiter::Types;
    let mut result = AllConstructors {
        types: BTreeMap::new(),
        functions: BTreeMap::new(),
        layer: 0,
    };

    for item in items {
        match item {
            Item::Delimiter(d) => current = d,
            Item::Constructor(c) => {
                match current {
                    Delimiter::Types => {
                        result.types.entry(c.output.namespace().unwrap().to_vec()) // FIXME
                            .or_insert_with(Default::default)
                            .entry(c.output.name().map(Into::into).unwrap()) // FIXME
                            .or_insert_with(Default::default)
                            .0.push(c);
                    },
                    Delimiter::Functions => {
                        result.functions.entry(c.variant.namespace().unwrap().to_vec()) // FIXME
                            .or_insert_with(Default::default)
                            .push(c);
                    },
                }
            },
            Item::Layer(i) => result.layer = i,
        }
    }

    result
}


#[derive(Debug, Default)]
struct Constructors(Vec<Constructor>);

#[derive(Debug)]
struct AllConstructors {
    types: BTreeMap<Vec<String>, BTreeMap<String, Constructors>>,
    functions: BTreeMap<Vec<String>, Vec<Constructor>>,
    layer: u32,
}

impl Constructors {
    fn fixup(&mut self, delim: Delimiter, fixup_map: &TypeFixupMap) {
        for c in &mut self.0 {
            c.fixup(delim, fixup_map);
        }
    }
}
