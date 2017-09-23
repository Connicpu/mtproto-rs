use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "printing")]
use quote;
use syn;

use ast::{Constructor, Delimiter, Item, Type, TypeFixupMap, TypeIrKind,
          no_conflict_ident, wrap_option_type, wrap_option_value};
use error;
use parser;


#[cfg(feature = "printing")]
pub fn generate_code_for(input: &str) -> quote::Tokens {
    let items = generate_items_for(input);

    quote! {
        #( #items )*
    }
}

pub fn generate_items_for(input: &str) -> Vec<syn::Item> {
    let constructors = {
        let mut items = parser::parse_string(input).unwrap();
        filter_items(&mut items);
        partition_by_delimiter_and_namespace(items)
    };

    let mut items = vec![
        syn::Item {
            ident: syn::Ident::new("LAYER"),
            vis: syn::Visibility::Public,
            attrs: vec![],
            node: syn::ItemKind::Const(
                Box::new(syn::Ty::Path(None, "i32".into())),
                Box::new(syn::Expr {
                    node: syn::ExprKind::Lit(syn::Lit::Int(constructors.layer as u64, syn::IntTy::Unsuffixed)),
                    attrs: vec![],
                })
            ),
        }
    ];

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
            .flat_map(|mut c| {
                c.fixup(Delimiter::Functions, &variants_to_outputs);
                //c.to_data_type_quoted().unwrap() // FIXME
                c.to_syn_data_type_items().unwrap() // FIXME
            });

        if namespaces.is_empty() {
            items.extend(substructs);
        } else {
            let mut namespaces_rev_iter = namespaces.into_iter().rev();

            let mut syn_mod = syn::Item {
                ident: syn::Ident::new(namespaces_rev_iter.next().unwrap()),
                vis: syn::Visibility::Public,
                attrs: vec![],
                node: syn::ItemKind::Mod(Some(substructs.collect())),
            };

            for namespace in namespaces_rev_iter {
                syn_mod = syn::Item {
                    ident: syn::Ident::new(namespace),
                    vis: syn::Visibility::Public,
                    attrs: vec![],
                    node: syn::ItemKind::Mod(Some(vec![syn_mod])),
                };
            }

            items.push(syn_mod);
        }
    }

    items
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

    fn to_syn_data_type_items(&self) -> error::Result<Vec<syn::Item>> {
        if self.0.len() == 1 {
            return self.0[0].to_syn_single_type_struct().map(|s| vec![s]);
        }

        assert!(self.0.len() >= 2); // FIXME: return errors instead of assert

        let name = self.0[0].output.name().map(no_conflict_ident).unwrap(); // FIXME
        let variants = self.0.iter().map(Constructor::to_syn_variant).collect();
        let methods = self.determine_methods(&name)?;
        let structs = self.0.iter()
            .map(Constructor::to_syn_variant_type_struct)
            .collect::<error::Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|maybe_struct| maybe_struct);

        let syn_enum = syn::Item {
            ident: name,
            vis: syn::Visibility::Public,
            attrs: vec![
                // Docs for syn 0.11.11 contain a bug: we need outer for #[..], not inner
                syn::Attribute {
                    style: syn::AttrStyle::Outer,
                    value: syn::MetaItem::List(
                        syn::Ident::new("derive"),
                        vec!["Clone", "Debug", "Serialize", "Deserialize", "MtProtoSized"]
                            .into_iter()
                            .map(|ident| syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(syn::Ident::new(ident))))
                            .collect(),
                    ),
                    is_sugared_doc: false,
                },
            ],
            // FIXME: in general case, generics can be present!
            node: syn::ItemKind::Enum(variants, syn::Generics {
                lifetimes: vec![],
                ty_params: vec![],
                where_clause: syn::WhereClause {
                    predicates: vec![],
                },
            }),
        };

        let syn_data_type_items = {
            // methods.len() == structs.len() == self.0.len()
            let mut v = Vec::with_capacity(1 + self.0.len() * 2);

            v.push(syn_enum);
            v.extend(methods);
            v.extend(structs);

            v
        };

        Ok(syn_data_type_items)
    }

    fn determine_methods(&self, enum_name: &syn::Ident) -> error::Result<Option<syn::Item>> {
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

            let method_name_no_conflict = no_conflict_ident(method_name);
            let mut type_ir = output_type.to_type_ir()?;

            let field_is_option = type_ir.needs_option();
            let exhaustive = constructors.len() == all_constructors_count;
            if !exhaustive {
                type_ir.with_option = true;
            }

            let force_option = !exhaustive && type_ir.kind == TypeIrKind::Unit;
            let field_access = syn::ExprKind::Field(
                Box::new(syn::ExprKind::Path(None, "x".into()).into()),
                method_name_no_conflict.clone(),
            ).into();

            let value = if field_is_option && type_ir.kind != TypeIrKind::Copyable {
                syn::ExprKind::MethodCall(syn::Ident::new("as_ref"), vec![], vec![field_access]).into()
            } else {
                let field_access = if type_ir.kind == TypeIrKind::Copyable {
                    field_access
                } else {
                    syn::ExprKind::AddrOf(syn::Mutability::Immutable, Box::new(field_access)).into()
                };

                let wrap = (type_ir.needs_option() && !field_is_option) || force_option;
                wrap_option_value(wrap, field_access)
            };

            let ty = wrap_option_type(force_option, type_ir.ref_type());
            let mut constructors_match_arms: Vec<syn::Arm> = constructors.into_iter()
                .map(|c| {
                    syn::Arm {
                        attrs: vec![],
                        pats: vec![
                            syn::Pat::TupleStruct(
                                syn::Path {
                                    global: false,
                                    segments: vec![
                                        enum_name.clone().into(),
                                        c.variant_name().into(),
                                    ],
                                },
                                vec![
                                    syn::Pat::Ident(
                                        syn::BindingMode::ByRef(syn::Mutability::Immutable),
                                        syn::Ident::new("x"),
                                        None,
                                    ),
                                ],
                                None,
                            ),
                        ],
                        guard: None,
                        body: Box::new(value.clone()),
                    }
                })
                .collect();

            if !exhaustive {
                let arm_ignore = syn::Arm {
                    attrs: vec![],
                    pats: vec![syn::Pat::Wild],
                    guard: None,
                    body: Box::new(syn::Expr {
                        node: syn::ExprKind::Path(None, "None".into()),
                        attrs: vec![],
                    }),
                };

                constructors_match_arms.push(arm_ignore);
            }

            let method = syn::ImplItem {
                ident: method_name_no_conflict,
                vis: syn::Visibility::Public,
                defaultness: syn::Defaultness::Final,
                attrs: vec![],
                node: syn::ImplItemKind::Method(
                    syn::MethodSig {
                        unsafety: syn::Unsafety::Normal,
                        constness: syn::Constness::NotConst,
                        abi: None,
                        decl: syn::FnDecl {
                            inputs: vec![
                                syn::FnArg::SelfRef(None, syn::Mutability::Immutable),
                            ],
                            output: syn::FunctionRetTy::Ty(ty),
                            variadic: false,
                        },
                        generics: Default::default(),
                    },
                    syn::Block {
                        stmts: vec![
                            syn::Stmt::Expr(Box::new(syn::ExprKind::Match(
                                Box::new(syn::ExprKind::Unary(
                                    syn::UnOp::Deref,
                                    Box::new(syn::ExprKind::Path(
                                        None,
                                        "self".into(),
                                    ).into()),
                                ).into()),
                                constructors_match_arms,
                            ).into())),
                        ],
                    },
                ),
            };

            methods.push(method);
        }

        let maybe_item = if methods.is_empty() {
            None
        } else {
            let item = syn::Item {
                ident: enum_name.clone(),
                vis: syn::Visibility::Inherited,
                attrs: vec![],
                node: syn::ItemKind::Impl(
                    syn::Unsafety::Normal,
                    syn::ImplPolarity::Positive,
                    syn::Generics::default(),
                    None,
                    Box::new(syn::Ty::Path(None, enum_name.clone().into())),
                    methods,
                ),
            };

            Some(item)
        };

        Ok(maybe_item)
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
