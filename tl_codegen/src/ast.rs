use std::collections::{BTreeMap, HashSet};

use quote;
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
            Type::Int => TypeIr::copyable(quote! { i32 }),
            Type::Named(ref v) => names_to_type_ir(v, &[])?,
            Type::TypeParameter(ref s) => {
                let resolved_ty_param = no_conflict_ident(s);
                TypeIr::noncopyable(quote! { #resolved_ty_param })
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
    pub(crate) tokens: quote::Tokens,
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
    pub fn copyable(tokens: quote::Tokens) -> TypeIr {
        TypeIr {
            tokens: tokens,
            with_option: false,
            type_ir_kind: TypeIrKind::Copyable,
        }
    }

    pub fn noncopyable(tokens: quote::Tokens) -> TypeIr {
        TypeIr {
            tokens: tokens,
            with_option: false,
            type_ir_kind: TypeIrKind::NonCopyable,
        }
    }

    pub fn needs_box(tokens: quote::Tokens) -> TypeIr {
        TypeIr {
            tokens: tokens,
            with_option: false,
            type_ir_kind: TypeIrKind::NeedsBox,
        }
    }

    pub fn unit() -> TypeIr {
        TypeIr {
            tokens: quote! { () },
            with_option: false,
            type_ir_kind: TypeIrKind::Unit,
        }
    }

    fn impl_unboxed(self) -> quote::Tokens {
        self.tokens
    }

    fn impl_boxed(self) -> quote::Tokens {
        if self.type_ir_kind == TypeIrKind::NeedsBox {
            let tokens = self.tokens;
            quote! { Box<#tokens> }
        } else {
            self.tokens
        }
    }

    fn impl_ref_type(self) -> quote::Tokens {
        let needs_ref = self.type_ir_kind != TypeIrKind::Copyable;
        let quoted = self.impl_boxed();

        if needs_ref {
            quote! { &#quoted }
        } else {
            quoted
        }
    }

    pub fn needs_option(&self) -> bool {
        self.with_option && self.type_ir_kind != TypeIrKind::Unit
    }

    pub fn unboxed(self) -> quote::Tokens {
        wrap_option_type(self.needs_option(), self.impl_unboxed())
    }

    pub fn boxed(self) -> quote::Tokens {
        wrap_option_type(self.needs_option(), self.impl_boxed())
    }

    pub fn ref_type(self) -> quote::Tokens {
        wrap_option_type(self.needs_option(), self.impl_ref_type())
    }

    pub fn ref_prefix(&self) -> quote::Tokens {
        if self.type_ir_kind == TypeIrKind::Copyable { quote! {} } else { quote! { ref } }
    }

    pub fn reference_prefix(&self) -> quote::Tokens {
        if self.type_ir_kind == TypeIrKind::Copyable { quote! {} } else { quote! { & } }
    }

    pub fn local_reference_prefix(&self) -> quote::Tokens {
        if self.type_ir_kind == TypeIrKind::Copyable { quote! { & } } else { quote! {} }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Field {
    pub(crate) name: Option<String>,
    pub(crate) ty: Type,
}

impl Field {
    fn to_quoted(&self) -> error::Result<quote::Tokens> {
        let ty = self.ty.to_type_ir()?.boxed();

        let field_quoted = match self.name {
            Some(ref name) => {
                let name = no_conflict_ident(name);

                quote! { #name: #ty }
            },
            None => {
                quote! { #ty }
            },
        };

        Ok(field_quoted)
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

    fn fields_tokens(&self) -> error::Result<quote::Tokens> {
        let tokens = if self.fields.is_empty() {
            quote! { ; }
        } else {
            let fields = self.non_flag_fields()
                .map(Field::to_quoted)
                .collect::<error::Result<Vec<_>>>()?;

            quote! {
                { #( pub #fields, )* }
            }
        };

        Ok(tokens)
    }

    fn variant_match_pattern_fields_ignored(&self) -> quote::Tokens {
        let name = self.variant_name();

        if self.fields.is_empty() {
            quote! { #name }
        } else {
            quote! { #name(..) }
        }
    }

    fn generics(&self) -> quote::Tokens {
        if self.type_parameters.is_empty() {
            return quote! {};
        }

        let types = self.type_parameters
            .iter()
            .map(|f| no_conflict_ident(f.name.as_ref().unwrap())); // FIXME .unwrap()

        quote! { <#(#types),*> }
    }

    /*fn rpc_generics(&self) -> quote::Tokens {
        
    }*/

    fn to_struct_quoted(&self, name: &syn::Ident) -> error::Result<quote::Tokens> {
        let generics = self.generics();
        let fields = self.fields_tokens()?;

        let mut derives = vec!["Debug", "Clone", "Serialize", "Deserialize", "MtProtoSized"];
        let mut id_attr = None;

        if let Some(tl_id) = self.tl_id {
            let id_formatted = format!("0x{:08x}", tl_id);

            derives.push("MtProtoIdentifiable");
            id_attr = Some(quote! { #[id = #id_formatted] })
        }

        let quoted = quote! {
            #[derive(#(#derives),*)]
            #id_attr
            pub struct #name #generics #fields
        };

        Ok(quoted)
    }

    // FIXME: fill in methods

    pub fn variant_name(&self) -> syn::Ident {
        self.variant.name().map(no_conflict_ident).unwrap() // FIXME: .unwrap()
    }

    pub fn to_variant_quoted(&self) -> quote::Tokens {
        let variant_name = self.variant_name();

        if self.fields.is_empty() {
            quote! { #variant_name }
        } else {
            quote! { #variant_name(#variant_name) }
        }
    }

    pub fn to_variant_type_struct_quoted(&self) -> error::Result<quote::Tokens> {
        if self.fields.is_empty() {
            Ok(quote! {})
        } else {
            self.to_type_struct_base_quoted(self.variant_name())
        }
    }

    fn to_type_struct_base_quoted(&self, name: syn::Ident) -> error::Result<quote::Tokens> {
        self.to_struct_quoted(&name)
    }

    pub fn to_single_type_struct_quoted(&self) -> error::Result<quote::Tokens> {
        let name = self.output.name().map(no_conflict_ident).unwrap(); // FIXME
        self.to_type_struct_base_quoted(name)
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

    fn tl_id(&self) -> Option<quote::Tokens> {
        self.tl_id.as_ref().map(|tl_id| {
            let id: syn::Ident = format!("0x{:08x}", tl_id).into();
            quote! { #id }
        })
    }
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


pub fn wrap_option_type(wrap: bool, ty: quote::Tokens) -> quote::Tokens {
    if wrap {
        quote! { Option<#ty> }
    } else {
        ty
    }
}

pub fn wrap_option_value(wrap: bool, ty: quote::Tokens) -> quote::Tokens {
    if wrap {
        quote! { Some(#ty) }
    } else {
        ty
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
                "Bool"   => TypeIr::copyable(quote! { bool }),
                "true"   => TypeIr::unit(),
                "int"    => TypeIr::copyable(quote! { i32 }),
                "long"   => TypeIr::copyable(quote! { i64 }),
                "int128" => TypeIr::copyable(quote! { ::extprim::i128::i128 }),
                "int256" => TypeIr::copyable(quote! { (::extprim::i128::i128, ::extprim::i128::i128) }),
                "double" => TypeIr::copyable(quote! { f64 }),
                "bytes"  => TypeIr::noncopyable(quote! { ::serde_bytes::ByteBuf }),
                "string" => TypeIr::noncopyable(quote! { String }),
                "vector" => {
                    let ty_param = get_ty_param()?.clone().unboxed();
                    TypeIr::noncopyable(quote! { Vec<#ty_param> })
                },
                "Vector" => {
                    let ty_param = get_ty_param()?.clone().unboxed();
                    TypeIr::noncopyable(quote! { Boxed<Vec<#ty_param>> })
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

    let ty = if type_parameters.len() == 0 {
        quote! { ::schema #(::#names)* }
    } else {
        let ty_params = type_parameters.into_iter().map(|ty_ir| ty_ir.clone().unboxed());
        quote! { ::schema #(::#names)* <#(#ty_params),*> }
    };

    // Special case two recursive types.
    let ty = match names.last().map(String::as_str) {
        Some("PageBlock") |
        Some("RichText") => TypeIr::needs_box(ty),
        _ => TypeIr::noncopyable(ty),
    };

    Ok(ty)
}
