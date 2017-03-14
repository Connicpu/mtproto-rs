#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use] extern crate quote;

use proc_macro::TokenStream;
use std::iter;

struct Impls {
    type_id: quote::Tokens,
    deserialize_bare: quote::Tokens,
    deserialize_as_bare: bool,
    extra_items: Vec<quote::Tokens>,
}

#[proc_macro_derive(TLType, attributes(tl_id))]
pub fn expand_tltype(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let ident = &ast.ident;
    let Impls {
        type_id, deserialize_bare, deserialize_as_bare, extra_items,
    } = match &ast.body {
        &syn::Body::Struct(ref body) => {
            impl_item_struct(extract_tl_id(&ast.attrs), &ident, body)
        },
        &syn::Body::Enum(ref variants) => impl_item_enum(&ident, variants),
    };

    let deserialize_as_bare = if deserialize_as_bare {
        quote! {
            fn deserialize<R: ::tl::parsing::Reader>(reader: &mut R) -> ::tl::Result<Self> {
                Self::deserialize_bare(None, reader)
            }
        }
    } else {
        quote!()
    };

    let ret = quote! {

        #(#extra_items)*

        impl ::tl::IdentifiableType for #ident {
            fn type_id(&self) -> Option<::tl::parsing::ConstructorId> {
                #type_id
            }
        }

        impl ::tl::ReadType for #ident {
            fn deserialize_bare<R: ::tl::parsing::Reader>(
                _id: Option<::tl::parsing::ConstructorId>,
                _reader: &mut R
            ) -> ::tl::Result<Self> {
                #deserialize_bare
            }

            #deserialize_as_bare
        }

    };
    ret.parse().unwrap()
}

fn extract_tl_id(attrs: &[syn::Attribute]) -> Option<u32> {
    attrs.into_iter()
        .filter_map(|a| {
            let items = match a {
                &syn::Attribute {
                    value: syn::MetaItem::List(_, ref items),
                    ..
                } if a.name() == "tl_id" => items,
                _ => return None,
            };
            let name = match items.first() {
                Some(&syn::NestedMetaItem::MetaItem(ref item)) if items.len() == 1 => item.name(),
                _ => return None,
            };
            if name.chars().next() != Some('_') {
                unimplemented!()
            }
            Some(u32::from_str_radix(&name[1..], 16).unwrap())
        })
        .next()
}

fn empty_variant(variant: &syn::VariantData) -> quote::Tokens {
    match variant {
        &syn::VariantData::Unit => quote! {},
        &syn::VariantData::Tuple(_) => quote! { (..) },
        &syn::VariantData::Struct(_) => quote! { {..} },
    }
}

fn deserialize_variant(variant: &syn::VariantData) -> quote::Tokens {
    let read_generic = iter::repeat(quote! { _reader.read_tl()? });
    match variant {
        &syn::VariantData::Unit => quote! {},
        &syn::VariantData::Tuple(ref fields_vec) => {
            let fields = read_generic.take(fields_vec.len());
            quote! {
                ( #( #fields ),* )
            }
        },
        &syn::VariantData::Struct(ref fields_vec) => {
            let fields = fields_vec.into_iter().map(|f| f.ident.as_ref().unwrap());
            quote! {
                { #( #fields: #read_generic ),* }
            }
        },
    }
}

fn impl_item_struct(tl_id_opt: Option<u32>, ty: &syn::Ident, body: &syn::VariantData) -> Impls {
    let deserialize_body = deserialize_variant(body);
    if let Some(tl_id) = tl_id_opt {
        let type_id = quote!(#ty::TYPE_ID);
        let extra_items = vec![quote! {

            impl #ty {
                const TYPE_ID: ::tl::parsing::ConstructorId = ::tl::parsing::ConstructorId(#tl_id);
            }

        }];
        Impls {
            type_id: quote! { Some(#type_id) },
            deserialize_bare: quote!(match _id {
                Some(#type_id) | None => Ok(#ty #deserialize_body),
                id => Err(::error::ErrorKind::InvalidType(vec![#type_id], id).into()),
            }),
            deserialize_as_bare: false,
            extra_items: extra_items,
        }
    } else {
        Impls {
            type_id: quote! { None },
            deserialize_bare: quote!(match _id {
                None => Ok(#ty #deserialize_body),
                id => Err(::error::ErrorKind::InvalidType(vec![], id).into()),
            }),
            deserialize_as_bare: true,
            extra_items: vec![],
        }
    }
}

fn impl_item_enum(ty: &syn::Ident, variants: &[syn::Variant]) -> Impls {
    let variant_names: Vec<_> = variants.iter()
        .map(|v| {
            let name = &v.ident;
            quote! { #ty::#name }
        })
        .collect();
    let tl_ids_: Vec<_> = variants.iter()
        .map(|v| extract_tl_id(&v.attrs).unwrap())
        .collect();
    let tl_ids = &tl_ids_;
    let empty_variants = variants.iter()
        .map(|v| empty_variant(&v.data));
    let type_id = {
        let variant_names = &variant_names;
        quote! {
            match self {
                #( &#variant_names #empty_variants => Some(::tl::parsing::ConstructorId(#tl_ids)), )*
            }
        }
    };

    let deserialize_fields = variants.iter()
        .map(|v| deserialize_variant(&v.data));
    let deserialize = {
        let variant_names = &variant_names;
        let tl_ids = &tl_ids;
        quote! {
            match _id {
                #( Some(#tl_ids) => Ok(#variant_names #deserialize_fields), )*
                id => Err(::error::ErrorKind::InvalidType(vec![#( #tl_ids ),*], id).into()),
            }
        }
    };

    Impls {
        type_id: type_id,
        deserialize_bare: deserialize,
        deserialize_as_bare: false,
        extra_items: vec![],
    }
}
