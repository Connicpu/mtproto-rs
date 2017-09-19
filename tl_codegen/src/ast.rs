use std::collections::{BTreeMap, HashSet};

use syn;
use synom;

use error::{self, ErrorKind};


pub type TypeFixupMap = BTreeMap<Vec<String>, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Int,
    Named(Vec<String>),
    TypeParameter(String),
    Generic(Vec<String>, Box<Type>),
    Flagged(String, u32, Box<Type>),
    Repeated(Vec<Field>),
}

impl Type {
    pub fn names_vec(&self) -> Option<&Vec<String>> {
        match *self {
            Type::Int |
            Type::TypeParameter(..) |
            Type::Flagged(..) |
            Type::Repeated(..) => None,
            Type::Named(ref v) |
            Type::Generic(ref v, ..) => Some(v),
        }
    }

    pub fn namespace(&self) -> Option<&[String]> {
        self.names_vec().map(|v| {
            // FIXME: do we really need assert here?
            assert!(v.len() >= 1);

            &v[..v.len()-1]
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.names_vec().and_then(|v| v.last().map(String::as_str))
    }

    pub fn flag_field(&self) -> Option<(&str, u32)> {
        match *self {
            Type::Flagged(ref f, b, _) => Some((f, b)),
            _ => None,
        }
    }

    pub fn is_type_parameter(&self) -> bool {
        match *self {
            Type::TypeParameter(..) => true,
            _ => false,
        }
    }

    fn fixup(&mut self, fixup_map: &TypeFixupMap) {
        // FIXME: what does `loc` variable mean?
        let loc = match *self {
            Type::Named(ref mut names) => names,
            Type::Generic(ref mut container, ref mut ty) => {
                ty.fixup(fixup_map);
                container
            },
            Type::Flagged(_, _, ref mut ty) => {
                ty.fixup(fixup_map);
                return;
            },
            _ => return,
        };
        match fixup_map.get(loc) {
            Some(replacement) => loc.clone_from(replacement),
            None => (),
        }
    }

    pub fn to_type_ir(&self) -> error::Result<TypeIr> {
        let type_ir = match *self {
            Type::Int => {
                let ty = syn::Ty::Path(None, syn::Path {
                    global: false,
                    segments: vec![
                        syn::PathSegment {
                            ident: syn::Ident::new("i32"),
                            parameters: syn::PathParameters::none(),
                        }
                    ],
                });

                TypeIr::copyable(ty)
            },
            Type::Named(ref v) => names_to_type_ir(v, &[])?,
            Type::TypeParameter(ref s) => {
                let ty = syn::Ty::Path(None, syn::Path {
                    global: false,
                    segments: vec![
                        syn::PathSegment {
                            ident: syn::Ident::new(no_conflict_ident(s)),
                            parameters: syn::PathParameters::none(),
                        }
                    ],
                });

                TypeIr::noncopyable(ty)
            },
            Type::Generic(ref container, ref ty) => {
                // TODO: change this to support multiple type parameters
                names_to_type_ir(container, &[ty.to_type_ir()?])?
            },
            Type::Flagged(_, _, ref ty) => {
                ty.to_type_ir()?
            },
            Type::Repeated(..) => unimplemented!(), // FIXME
        };

        Ok(type_ir)
    }
}

#[derive(Clone, Debug)]
pub struct TypeIr {
    pub(crate) ty: syn::Ty,
    pub(crate) with_option: bool,
    pub(crate) type_ir_kind: TypeIrKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeIrKind {
    Copyable,
    NonCopyable,
    NeedsBox,
    Unit,
}

impl TypeIr {
    pub fn copyable(ty: syn::Ty) -> TypeIr {
        TypeIr {
            ty: ty,
            with_option: false,
            type_ir_kind: TypeIrKind::Copyable,
        }
    }

    pub fn noncopyable(ty: syn::Ty) -> TypeIr {
        TypeIr {
            ty: ty,
            with_option: false,
            type_ir_kind: TypeIrKind::NonCopyable,
        }
    }

    pub fn needs_box(ty: syn::Ty) -> TypeIr {
        TypeIr {
            ty: ty,
            with_option: false,
            type_ir_kind: TypeIrKind::NeedsBox,
        }
    }

    pub fn unit() -> TypeIr {
        TypeIr {
            ty: syn::Ty::Tup(vec![]),
            with_option: false,
            type_ir_kind: TypeIrKind::Unit,
        }
    }

    fn impl_unboxed(self) -> syn::Ty {
        self.ty
    }

    fn impl_boxed(self) -> syn::Ty {
        if self.type_ir_kind == TypeIrKind::NeedsBox {
            syn::Ty::Path(None, syn::Path {
                global: false,
                segments: vec![
                    syn::PathSegment {
                        ident: syn::Ident::new("Box"),
                        parameters: syn::PathParameters::AngleBracketed(syn::AngleBracketedParameterData {
                            lifetimes: vec![],
                            types: vec![self.ty],
                            bindings: vec![],
                        }),
                    },
                ],
            })
        } else {
            self.ty
        }
    }

    fn impl_ref_type(self) -> syn::Ty {
        let needs_ref = self.type_ir_kind != TypeIrKind::Copyable;
        let syn_ty = self.impl_boxed();

        if needs_ref {
            syn::Ty::Rptr(None, Box::new(syn::MutTy {
                ty: syn_ty,
                mutability: syn::Mutability::Immutable,
            }))
        } else {
            syn_ty
        }
    }

    pub fn needs_option(&self) -> bool {
        self.with_option && self.type_ir_kind != TypeIrKind::Unit
    }

    pub fn unboxed(self) -> syn::Ty {
        wrap_option_type(self.needs_option(), self.impl_unboxed())
    }

    pub fn boxed(self) -> syn::Ty {
        wrap_option_type(self.needs_option(), self.impl_boxed())
    }

    pub fn ref_type(self) -> syn::Ty {
        wrap_option_type(self.needs_option(), self.impl_ref_type())
    }

    /*pub fn ref_prefix(&self) -> syn::Ty {
        if self.type_ir_kind == TypeIrKind::Copyable { quote! {} } else { quote! { ref } }
    }

    pub fn reference_prefix(&self) -> syn::Ty {
        if self.type_ir_kind == TypeIrKind::Copyable { quote! {} } else { quote! { & } }
    }

    pub fn local_reference_prefix(&self) -> syn::Ty {
        if self.type_ir_kind == TypeIrKind::Copyable { quote! { & } } else { quote! {} }
    }*/
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Field {
    pub(crate) name: Option<String>,
    pub(crate) ty: Type,
}

impl Field {
    fn to_syn_field(&self) -> error::Result<syn::Field> {
        let ty = self.ty.to_type_ir()?.boxed();

        let mut field = syn::Field {
            ident: None,
            vis: syn::Visibility::Inherited,
            attrs: vec![],
            ty: ty,
        };

        if let Some(ref name) = self.name {
            field.ident = Some(syn::Ident::new(no_conflict_ident(name)));
        }

        Ok(field)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Constructor {
    pub(crate) variant: Type,
    pub(crate) tl_id: Option<u32>,
    pub(crate) type_parameters: Vec<Field>,
    pub(crate) fields: Vec<Field>,
    pub(crate) output: Type,
}

impl Constructor {
    pub fn fixup(&mut self, which: Delimiter, fixup_map: &TypeFixupMap) {
        if which == Delimiter::Functions {
            self.fixup_output();
        }

        self.fixup_fields(fixup_map);
        self.fixup_variant();
    }

    fn fixup_output(&mut self) {
        let is_output_a_type_parameter = |constructor: &Constructor| {
            let output_name = match constructor.output {
                Type::Named(ref v) if v.len() == 1 => v[0].as_str(),
                _ => return false,
            };

            for p in &constructor.type_parameters {
                if p.name.as_ref().map(String::as_str) == Some(output_name) {
                    return true;
                }
            }

            false
        };

        if is_output_a_type_parameter(self) {
            self.output = Type::TypeParameter(self.output.name().unwrap().into()) // FIXME
        }
    }

    fn fixup_fields(&mut self, fixup_map: &TypeFixupMap) {
        for f in &mut self.fields {
            f.ty.fixup(fixup_map);
        }

        match self.variant.name() {
            Some("resPQ") |
            Some("p_q_inner_data") |
            Some("p_q_inner_data_temp") |
            Some("server_DH_params_ok") |
            Some("server_DH_inner_data") |
            Some("client_DH_inner_data") |
            Some("req_DH_params") |
            Some("set_client_DH_params") => (),
            _ => return,
        }

        for f in &mut self.fields {
            match f.ty {
                Type::Named(ref mut v) if v.len() == 1 && v[0] == "string" => {
                    v[0] = "bytes".into();
                },
                _ => (),
            }
        }
    }

    fn fixup_variant(&mut self) {
        match self.variant.name() {
            // The 'updates' variant struct conflicts with the module.
            Some("updates") => {
                self.variant = Type::Named(vec!["updates_".into()]);
            },
            _ => (),
        }
    }

    fn flag_field_names(&self) -> HashSet<&str> {
        self.fields
            .iter()
            .filter_map(|f| {
                f.ty.flag_field().map(|(flag, _)| flag)
            })
            .collect()
    }

    pub fn non_flag_fields<'a>(&'a self) -> Box<Iterator<Item = &'a Field> + 'a> {
        let flag_fields = self.flag_field_names();

        Box::new({
            self.fields
                .iter()
                .filter(move |f| {
                    f.name.as_ref()
                        .map(|s| !flag_fields.contains(s.as_str()))
                        .unwrap_or(true)
                })
        })
    }

    fn variant_match_pattern_fields_ignored(&self) -> syn::Pat {
        let path = syn::Path {
            global: false,
            segments: vec![
                syn::PathSegment {
                    ident: syn::Ident::new(self.variant_name()),
                    parameters: syn::PathParameters::none(),
                }
            ],
        };

        syn::Pat::TupleStruct(path, vec![], Some(0))
    }

    fn syn_generics(&self) -> syn::Generics {
        let ty_params = self.type_parameters.iter()
            .map(|field| syn::TyParam {
                attrs: vec![],
                ident: syn::Ident::new(field.name.clone().unwrap()), // FIXME
                bounds: vec![],
                default: None,
            })
            .collect();

        syn::Generics {
            lifetimes: vec![],
            ty_params: ty_params,
            where_clause: syn::WhereClause {
                predicates: vec![],
            },
        }
    }

    /*fn rpc_generics(&self) -> quote::Tokens {
        
    }*/

    fn to_syn_struct(&self, name: &syn::Ident) -> error::Result<syn::Item> {
        let syn_generics = self.syn_generics();
        let syn_fields = self.fields
            .iter()
            .map(Field::to_syn_field)
            .collect::<error::Result<_>>()?;

        let mut derives = vec!["Clone", "Debug", "Serialize", "Deserialize", "MtProtoSized"];
        let mut id_attr = None;

        if let Some(tl_id) = self.tl_id {
            derives.push("MtProtoIdentifiable");
            id_attr = Some(syn::Attribute {
                style: syn::AttrStyle::Outer,
                value: syn::MetaItem::NameValue(
                    syn::Ident::new("id"),
                    syn::Lit::Str(format!("0x{:08x}", tl_id), syn::StrStyle::Cooked),
                ),
                is_sugared_doc: false,
            });
        }

        let derive_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            value: syn::MetaItem::List(
                syn::Ident::new("derive"),
                derives.into_iter()
                    .map(|ident| syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(syn::Ident::new(ident))))
                    .collect(),
            ),
            is_sugared_doc: false,
        };

        let attrs = if let Some(id_attr) = id_attr {
            vec![derive_attr, id_attr]
        } else {
            vec![derive_attr]
        };

        let syn_struct = syn::Item {
            ident: name.clone(),
            vis: syn::Visibility::Public,
            attrs: attrs,
            node: syn::ItemKind::Struct(syn::VariantData::Struct(syn_fields), syn_generics),
        };

        Ok(syn_struct)
    }

    // FIXME: fill in methods

    pub fn variant_name(&self) -> syn::Ident {
        self.variant.name().map(no_conflict_ident).unwrap() // FIXME: .unwrap()
    }

    pub fn to_syn_variant(&self) -> syn::Variant {
        let variant_name = self.variant_name();

        let variant_data = if self.fields.is_empty() {
            syn::VariantData::Unit
        } else {
            syn::VariantData::Tuple(vec![
                syn::Field {
                    ident: None,
                    vis: syn::Visibility::Inherited,
                    attrs: vec![],
                    ty: syn::Ty::Path(None, syn::Path {
                        global: false,
                        segments: vec![
                            syn::PathSegment {
                                ident: syn::Ident::new(variant_name.clone()),
                                parameters: syn::PathParameters::none(),
                            }
                        ],
                    }),
                }
            ])
        };

        syn::Variant {
            ident: syn::Ident::new(variant_name),
            attrs: vec![],
            data: variant_data,
            discriminant: None,
        }
    }

    pub fn to_syn_variant_type_struct(&self) -> error::Result<Option<syn::Item>> {
        if self.fields.is_empty() {
            Ok(None)
        } else {
            self.to_syn_type_struct_base(self.variant_name()).map(Some)
        }
    }

    fn to_syn_type_struct_base(&self, name: syn::Ident) -> error::Result<syn::Item> {
        self.to_syn_struct(&name)
    }

    pub fn to_syn_single_type_struct(&self) -> error::Result<syn::Item> {
        let name = self.output.name().map(no_conflict_ident).unwrap(); // FIXME
        self.to_syn_type_struct_base(name)
    }

    /*fn to_variant_def_destructure(&self, name: &syn::Ident) -> Option<quote::Tokens> {
        if self.fields.is_empty() {
            return None;
        }

        let fields = self.non_flag_fields()
            .map(|f| {
                let prefix = f.ty.to_type_ir()?.ref_prefix();
                let name = no_conflict_ident(f.name.as_ref().unwrap()); // FIXME
                quote! { #prefix #name }
            })
            .collect::<error::Result<Vec<_>>>()?;

        Some(quote! {
            #name { #( #fields ),* }
        })
    }*/

    /*fn tl_id(&self) -> Option<quote::Tokens> {
        self.tl_id.as_ref().map(|tl_id| {
            let id: syn::Ident = format!("0x{:08x}", tl_id).into();
            quote! { #id }
        })
    }*/
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Delimiter {
    Types,
    Functions,
}

#[derive(Debug, Clone)]
pub enum Item {
    Delimiter(Delimiter),
    Constructor(Constructor),
    Layer(u32),
}


pub fn wrap_option_type(wrap: bool, ty: syn::Ty) -> syn::Ty {
    if wrap {
        syn::Ty::Path(None, syn::Path {
            global: false,
            segments: vec![
                syn::PathSegment {
                    ident: syn::Ident::new("Option"),
                    parameters: syn::PathParameters::AngleBracketed(syn::AngleBracketedParameterData {
                        lifetimes: vec![],
                        types: vec![ty],
                        bindings: vec![],
                    }),
                },
            ],
        })
    } else {
        ty
    }
}

pub fn wrap_option_value(wrap: bool, value: syn::Expr) -> syn::Expr {
    if wrap {
        syn::Expr {
            node: syn::ExprKind::Call(
                Box::new(syn::Expr {
                    node: syn::ExprKind::Path(None, syn::Path {
                        global: false,
                        segments: vec![
                            syn::PathSegment {
                                ident: syn::Ident::new("Some"),
                                parameters: syn::PathParameters::none(),
                            },
                        ],
                    }),
                    attrs: vec![],
                }),
                vec![value],
            ),
            attrs: vec![],
        }
    } else {
        value
    }
}

pub fn no_conflict_ident(s: &str) -> syn::Ident {
    let mut candidate: String = s.into();

    loop {
        match syn::parse::ident(&candidate) {
            synom::IResult::Done("", id) => return id,
            _ => candidate.push('_'),
        }
    }
}

fn names_to_type_ir(names: &[String], type_parameters: &[TypeIr]) -> error::Result<TypeIr> {
    if names.len() == 1 {
        let get_ty_param = || -> error::Result<_> {
            if type_parameters.len() != 1 {
                bail!(ErrorKind::WrongTyParamsCount(type_parameters.to_vec(), 1));
            }

            Ok(&type_parameters[0])
        };

        let handle_simple_types = || -> error::Result<_> {
            let type_ir = match names[0].as_str() {
                "Bool"   => TypeIr::copyable(syn::parse_type(quote! { bool }.as_str()).unwrap()),
                "true"   => TypeIr::unit(),
                "int"    => TypeIr::copyable(syn::parse_type(quote! { i32 }.as_str()).unwrap()),
                "long"   => TypeIr::copyable(syn::parse_type(quote! { i64 }.as_str()).unwrap()),
                "int128" => TypeIr::copyable(syn::parse_type(quote! { ::extprim::i128::i128 }.as_str()).unwrap()),
                "int256" => TypeIr::copyable(syn::parse_type(quote! { (::extprim::i128::i128, ::extprim::i128::i128) }.as_str()).unwrap()),
                "double" => TypeIr::copyable(syn::parse_type(quote! { f64 }.as_str()).unwrap()),
                "bytes"  => TypeIr::noncopyable(syn::parse_type(quote! { ::serde_bytes::ByteBuf }.as_str()).unwrap()),
                "string" => TypeIr::noncopyable(syn::parse_type(quote! { String }.as_str()).unwrap()),
                "vector" => {
                    let ty_param = get_ty_param()?.clone().unboxed();
                    TypeIr::noncopyable(syn::parse_type(quote! { Vec<#ty_param> }.as_str()).unwrap())
                },
                "Vector" => {
                    let ty_param = get_ty_param()?.clone().unboxed();
                    TypeIr::noncopyable(syn::parse_type(quote! { ::serde_mtproto::Boxed<Vec<#ty_param>> }.as_str()).unwrap())
                },
                _ => return Ok(None),
            };

            Ok(Some(type_ir))
        };

        match handle_simple_types()? {
            Some(type_ir) => return Ok(type_ir),
            None          => (),
        }
    }

    let names_as_idents = names.iter().map(|name| syn::Ident::new(name.as_str()));
    let ty = if type_parameters.len() == 0 {
        syn::parse_type(quote! { ::schema #(::#names_as_idents)* }.as_str()).unwrap()
    } else {
        let ty_params = type_parameters.into_iter().map(|ty_ir| ty_ir.clone().unboxed());
        syn::parse_type(quote! { ::schema #(::#names_as_idents)* <#(#ty_params),*> }.as_str()).unwrap()
    };

    // Special case two recursive types.
    let ty = match names.last().map(String::as_str) {
        Some("PageBlock") |
        Some("RichText") => TypeIr::needs_box(ty),
        _ => TypeIr::noncopyable(ty),
    };

    Ok(ty)
}
