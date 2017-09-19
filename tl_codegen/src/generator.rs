use std::collections::{BTreeMap, BTreeSet};

use quote;
use syn;

use ast::{Constructor, Delimiter, Item, Type, TypeFixupMap, TypeIr, TypeIrKind,
          no_conflict_ident, wrap_option_type, wrap_option_value};
use error;
use parser;


pub fn generate_code_for(input: &str) -> quote::Tokens {
    let constructors = {
        let mut items = parser::parse_string(input).unwrap();
        filter_items(&mut items);
        partition_by_delimiter_and_namespace(items)
    };

    let layer = constructors.layer as i32;
    let mut items = vec![quote! {
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

    for (namespaces, mut constructor_map) in constructors.types {
        let substructs = constructor_map.values_mut()
            .map(|mut c| {
                c.fixup(Delimiter::Functions, &variants_to_outputs);
                c.to_data_type_quoted().unwrap() // FIXME
            });

        let mut quoted = quote! { #(#substructs)* };
        for namespace in namespaces.into_iter().rev() {
            quoted = quote! {
                pub mod #namespace {
                    #quoted
                }
            };
        }

        items.push(quoted);
    }

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

    fn to_data_type_quoted(&self) -> error::Result<quote::Tokens> {
        if self.0.len() == 1 {
            return self.0[0].to_single_type_struct_quoted();
        }

        assert!(self.0.len() >= 2); // FIXME: return errors instead of assert

        let name = self.0[0].output.name().map(no_conflict_ident).unwrap(); // FIXME
        let variants = self.0.iter().map(Constructor::to_variant_quoted);
        let methods = self.determine_methods(&name)?;
        let structs = self.0.iter()
            .map(Constructor::to_variant_type_struct_quoted)
            .collect::<error::Result<Vec<_>>>()?;

        let data_type_quoted = quote! {
            #[derive(Debug, Clone)]
            pub enum #name {
                #( #variants, )*
            }

            #methods
            #( #structs )*
        };

        Ok(data_type_quoted)
    }

    fn determine_methods(&self, enum_name: &syn::Ident) -> error::Result<quote::Tokens> {
        let all_constructors_count = self.0.len();
        let mut methods = vec![];

        for (method_name, typemap) in self.coalesce_methods() {
            if typemap.len() != 1 {
                continue;
            }

            // FIXME: handle case when typemap.len() == 0
            let (output_type, constructors) = typemap.into_iter().next().unwrap();
            if constructors.len() <= 1 {
                //panic!("{:#?}", constructors);
                continue;
            }

            let method_name = no_conflict_ident(method_name);
            let mut type_ir = output_type.to_type_ir()?;

            let field_is_option = type_ir.needs_option();
            let exhaustive = constructors.len() == all_constructors_count;
            if !exhaustive {
                type_ir.with_option = true;
            }

            let force_option = !exhaustive && type_ir.type_ir_kind == TypeIrKind::Unit;
            let value = if field_is_option && type_ir.type_ir_kind != TypeIrKind::Copyable {
                quote! { x.#method_name.as_ref() }
            } else {
                let ref_ = type_ir.reference_prefix();
                let wrap = (type_ir.needs_option() && !field_is_option) || force_option;
                wrap_option_value(wrap, quote! { #ref_ x.#method_name })
            };

            let ty = wrap_option_type(force_option, type_ir.ref_type());
            let constructors = constructors.into_iter()
                .map(|c| {
                    let constructor_name = c.variant_name();

                    quote! {
                        #enum_name::#constructor_name(ref x) => #value
                    }
                });

            let trailer_non_exhaustive = if exhaustive {
                None
            } else {
                Some(quote! { _ => None })
            };

            methods.push(quote! {
                pub fn #method_name(&self) -> #ty {
                    match *self {
                        #( #constructors, )*
                        #trailer_non_exhaustive
                    }
                }
            });
        }

        let methods_tokens = if methods.is_empty() {
            quote! {}
        } else {
            quote! {
                impl #enum_name {
                    #( #methods )*
                }
            }
        };

        Ok(methods_tokens)
    }

    fn coalesce_methods(&self) -> BTreeMap<&str, BTreeMap<&Type, BTreeSet<&Constructor>>> {
        let mut map: BTreeMap<_, BTreeMap<_, BTreeSet<_>>> = BTreeMap::new();

        for constructor in &self.0 {
            for field in constructor.non_flag_fields() {
                let name = match field.name.as_ref() {
                    Some(s) => s.as_str(),
                    None => continue,
                };

                map.entry(name)
                    .or_insert_with(Default::default)
                    .entry(&field.ty)
                    .or_insert_with(Default::default)
                    .insert(constructor);
            }
        }

        map
    }
}
