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

    fn to_quoted(&self) -> error::Result<quote::Tokens> {
        let quoted = match *self {
            Type::Int => quote! { i32 },
            Type::Named(ref v) => names_to_quoted(v, &[])?,
            Type::TypeParameter(ref s) => {
                let resolved_ty_param = no_conflict_ident(s);
                quote! { #resolved_ty_param }
            },
            Type::Generic(ref container, ref ty) => {
                // TODO: change this to support multiple type parameters
                names_to_quoted(container, &[ty.to_quoted()?])?
            },
            Type::Flagged(_, _, ref ty) => {
                ty.to_quoted()?
            },
            Type::Repeated(..) => unimplemented!(), // FIXME
        };

        Ok(quoted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Field {
    pub(crate) name: Option<String>,
    pub(crate) ty: Type,
}

impl Field {
    fn to_quoted(&self) -> error::Result<quote::Tokens> {
        let ty = self.ty.to_quoted()?;

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
        //if self.is_output_a_type_parameter() {
            // FIXME
        //}

        unimplemented!()
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

    fn non_flag_fields<'a>(&'a self) -> Box<Iterator<Item = &'a Field> + 'a> {
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

        let mut derives = vec!["Debug", "Clone", "Serialize", "Deserialize"];
        let mut id_attr = None;

        if let Some(tl_id) = self.tl_id {
            let id_formatted = "0x{:08x}";

            derives.push("Identifiable");
            id_attr = Some(quote! { #[id = #id_formatted] })
        }

        // FIXME: add MtProtoIdentifiable and MtProtoSized derives
        let quoted = quote! {
            #[derive(#(#derives),*)]
            #id_attr
            pub struct #name #generics #fields
        };

        Ok(quoted)
    }

    // FIXME: fill in methods

    fn variant_name(&self) -> syn::Ident {
        self.variant.name().map(no_conflict_ident).unwrap() // FIXME: .unwrap()
    }

    fn tl_id(&self) -> Option<quote::Tokens> {
        self.tl_id.as_ref().map(|tl_id| {
            let id: syn::Ident = format!("0x{:08x}", tl_id).into();
            quote! { #tl_id }
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


fn no_conflict_ident(s: &str) -> syn::Ident {
    let mut candidate: String = s.into();

    loop {
        match syn::parse::ident(&candidate) {
            synom::IResult::Done("", id) => return id,
            _ => candidate.push('_'),
        }
    }
}

fn names_to_quoted(names: &[String], type_parameters: &[quote::Tokens]) -> error::Result<quote::Tokens> {
    if names.len() == 1 {
        let get_ty_param = || -> error::Result<_> {
            if type_parameters.len() != 1 {
                bail!(ErrorKind::WrongTyParamsCount(type_parameters.to_vec(), 1));
            }

            Ok(&type_parameters[0])
        };

        let handle_simple_types = || -> error::Result<_> {
            let ty = match names[0].as_str() {
                "Bool"   => quote! { bool },
                "true"   => quote! { bool },
                "int"    => quote! { i32 },
                "long"   => quote! { i64 },
                "int128" => quote! { ::extprim::i128::i128 },
                "int256" => quote! { (::extprim::i128::i128, ::extprim::i128::i128) },
                "double" => quote! { f64 },
                "bytes"  => quote! { ::serde_bytes::ByteBuf },
                "string" => quote! { String },
                "vector" => {
                    let ty_param = get_ty_param()?;
                    quote! { Vec<#ty_param> }
                },
                "Vector" => {
                    let ty_param = get_ty_param()?;
                    quote! { Boxed<Vec<#ty_param>> }
                },
                _ => return Ok(None),
            };

            Ok(Some(ty))
        };

        match handle_simple_types()? {
            Some(ty) => return Ok(ty),
            None => (),
        }
    }

    let ty = if type_parameters.len() == 0 {
        quote! { ::schema #(::#names)* }
    } else {
        quote! { ::schema #(::#names)* <#(#type_parameters),*> }
    };

    // Special case two recursive types.
    let ty = match names.last().map(String::as_str) {
        Some("PageBlock") |
        Some("RichText") => quote! { Box<#ty> },
        _ => ty,
    };

    Ok(ty)
}
