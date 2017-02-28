#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quote;

use proc_macro::TokenStream;
use std::sync::Mutex;
use std::iter;

struct Impls {
    bare_type: quote::Tokens,
    type_id: quote::Tokens,
    serialize: quote::Tokens,
    deserialize: quote::Tokens,
    deserialize_boxed: quote::Tokens,
    extra_items: Vec<quote::Tokens>,
}

fn add_any_bounds(generics: &syn::Generics) -> syn::Generics {
    let mut ret = generics.clone();
    for ty in &mut ret.ty_params {
        ty.bounds.push(syn::TyParamBound::Trait(
            syn::PolyTraitRef {
                bound_lifetimes: vec![],
                trait_ref: syn::parse_path(&quote! { ::std::any::Any }.to_string()).unwrap(),
            },
            syn::TraitBoundModifier::None
        ));
    }
    ret
}

lazy_static! {
    static ref TYPE_COUNT: Mutex<usize> = Mutex::new(0);
}

fn increment_type_counts() -> (syn::Ident, syn::Ident) {
    let mut handle = TYPE_COUNT.lock().unwrap();
    *handle += 1;
    (format!("__register_{}", *handle).into(),
     format!("__register_{}", *handle + 1).into())
}

fn next_registration(body: quote::Tokens) -> quote::Tokens {
    let (cur, next) = increment_type_counts();
    quote! {

        impl ::AllDynamicTypes {
            #[inline(always)]
            pub fn #cur<R: ::tl::parsing::Reader>(cstore: &mut ::tl::dynamic::TLCtorMap<R>) {
                #body
                ::AllDynamicTypes::#next(cstore)
            }
        }

    }
}

#[proc_macro_derive(TLType, attributes(tl_id))]
pub fn expand_tltype(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let ident = &ast.ident;
    let generics = add_any_bounds(&ast.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let Impls {
        bare_type, type_id, serialize, deserialize, deserialize_boxed, extra_items,
    } = match &ast.body {
        &syn::Body::Struct(ref body) => {
            impl_item_struct(extract_tl_id(&ast.attrs), &ident, &generics, body)
        },
        &syn::Body::Enum(ref variants) => impl_item_enum(&ident, variants),
    };

    let ret = quote! {

        #(#extra_items)*

        impl #impl_generics ::tl::Type for #ident #ty_generics #where_clause {
            #[inline]
            fn bare_type() -> bool {
                #bare_type
            }

            #[inline]
            fn type_id(&self) -> Option<::tl::parsing::ConstructorId> {
                #type_id
            }

            fn serialize<W: ::tl::parsing::Writer>(
                &self,
                _writer: &mut W
            ) -> ::tl::Result<()> {
                #serialize
            }

            fn deserialize<R: ::tl::parsing::Reader>(
                _reader: &mut R
            ) -> ::tl::Result<Self> {
                #deserialize
            }

            fn deserialize_boxed<R: ::tl::parsing::Reader>(
                _id: ::tl::parsing::ConstructorId,
                _reader: &mut R
            ) -> ::tl::Result<Self> {
                #deserialize_boxed
            }
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
    let read_generic = iter::repeat(quote! { _reader.read_generic()? });
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

fn ty_turbofish(ty: &syn::Ident, ty_generics: &syn::TyGenerics) -> quote::Tokens {
    let tokens = quote! { #ty_generics };
    if tokens.as_str().is_empty() {
        quote! { #ty }
    } else {
        quote! { #ty :: #tokens }
    }
}

fn impl_item_struct(tl_id_opt: Option<u32>, ty: &syn::Ident, generics: &syn::Generics, body: &syn::VariantData) -> Impls {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let ty_turbofish = ty_turbofish(ty, &ty_generics);
    let boxes = if generics.ty_params.is_empty() {
        quote! {}
    } else {
        let boxes = (0..generics.ty_params.len())
            .into_iter()
            .map(|_| quote! { Box<::tl::dynamic::TLObject> });
        quote! { <#( #boxes ),*> }
    };

    let serialize = match body {
        &syn::VariantData::Unit => quote! {},
        &syn::VariantData::Tuple(ref fields_vec) => {
            let fields = (0..fields_vec.len()).map(Into::<syn::Ident>::into);
            quote! { #(
                _writer.write_generic(&self.#fields)?;
            )* }
        },
        &syn::VariantData::Struct(ref fields_vec) => {
            let fields = fields_vec.into_iter().map(|f| f.ident.as_ref().unwrap());
            quote! { #(
                _writer.write_generic(&self.#fields)?;
            )* }
        },
    };
    let serialize = quote! {
        { #serialize }
        Ok(())
    };

    let deserialize_body = deserialize_variant(body);
    let deserialize = quote! { Ok(#ty #deserialize_body) };

    if let Some(tl_id) = tl_id_opt {
        let type_id = quote! {
            #ty_turbofish::TYPE_ID
        };
        let tlctor = quote! {
            cstore.0.insert(::tl::parsing::ConstructorId(#tl_id), ::tl::dynamic::TLCtor(<#ty #boxes as ::tl::dynamic::TLDynamic>::deserialize));
        };
        let extra_items = vec![quote! {

            impl #impl_generics #ty #ty_generics #where_clause {
                const TYPE_ID: ::tl::parsing::ConstructorId = ::tl::parsing::ConstructorId(#tl_id);
            }

        }, next_registration(tlctor)];
        let deserialize_boxed = quote! {
            if _id == #type_id {
                Self::deserialize(_reader)
            } else {
                Err(::error::ErrorKind::InvalidType(_id).into())
            }
        };
        Impls {
            bare_type: quote! { false },
            type_id: quote! { Some(#type_id) },
            serialize: serialize,
            deserialize: deserialize,
            deserialize_boxed: deserialize_boxed,
            extra_items: extra_items,
        }
    } else {
        Impls {
            bare_type: quote! { true },
            type_id: quote! { None },
            serialize: serialize,
            deserialize: deserialize,
            deserialize_boxed: quote! { Err(::error::ErrorKind::PrimitiveAsPolymorphic.into()) },
            extra_items: vec![],
        }
    }
}

fn serialize_enum_variant(ty: &syn::Ident, variant: &syn::Variant) -> quote::Tokens {
    let name = &variant.ident;
    match &variant.data {
        &syn::VariantData::Unit => quote! {
            &#ty::#name => (),
        },
        &syn::VariantData::Tuple(ref fields_vec) => {
            let fields_vec: Vec<syn::Ident> = (0..fields_vec.len())
                .map(|i| format!("t{}", i).into())
                .collect();
            let fields = &fields_vec;
            quote! {
                &#ty::#name( #( ref #fields ),* ) => {
                    #( _writer.write_generic(#fields)?; )*
                },
            }
        },
        &syn::VariantData::Struct(ref fields_vec) => {
            let fields_vec: Vec<&syn::Ident> = fields_vec
                .into_iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            let fields = &fields_vec;
            quote! {
                &#ty::#name { #( ref #fields ),* } => {
                    #( _writer.write_generic(#fields)?; )*
                },
            }
        },
    }
}

fn impl_item_enum(ty: &syn::Ident, variants: &[syn::Variant]) -> Impls {
    let variant_names: Vec<_> = variants.iter()
        .map(|v| {
            let name = &v.ident;
            quote! { #ty::#name }
        })
        .collect();
    let tl_ids: Vec<_> = variants.iter()
        .map(|v| extract_tl_id(&v.attrs).unwrap())
        .collect();
    let empty_variants = variants.iter()
        .map(|v| empty_variant(&v.data));
    let type_id = {
        let variant_names = &variant_names;
        let tl_ids = &tl_ids;
        quote! {
            match self {
                #( &#variant_names #empty_variants => Some(::tl::parsing::ConstructorId(#tl_ids)), )*
            }
        }
    };

    let serialize = {
        let serialize_arm = variants.iter()
            .map(|v| serialize_enum_variant(ty, v));
        quote! {
            match self {
                #( #serialize_arm )*
            }
            Ok(())
        }
    };

    let deserialize = quote! {
        Err(::error::ErrorKind::BoxedAsBare.into())
    };

    let deserialize_fields = variants.iter()
        .map(|v| deserialize_variant(&v.data));
    let deserialize_boxed = {
        let variant_names = &variant_names;
        let tl_ids = &tl_ids;
        quote! {
            match _id.0 {
                #( #tl_ids => Ok(#variant_names #deserialize_fields), )*
                id => Err(::error::ErrorKind::InvalidType(::tl::parsing::ConstructorId(id)).into()),
            }
        }
    };

    let ty_repeated = iter::repeat(ty);
    let extra_items = vec![next_registration(quote! {
        #( cstore.0.insert(::tl::parsing::ConstructorId(#tl_ids), ::tl::dynamic::TLCtor(<#ty_repeated as ::tl::dynamic::TLDynamic>::deserialize)); )*
    })];

    Impls {
        bare_type: quote! { false },
        type_id: type_id,
        serialize: serialize,
        deserialize: deserialize,
        deserialize_boxed: deserialize_boxed,
        extra_items: extra_items,
    }
}

#[proc_macro_derive(TLDynamic, attributes(tl_register_all))]
pub fn expand_tldynamic(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input(&input.to_string()).unwrap();
    let ident = &ast.ident;
    let (ty_count, _) = increment_type_counts();
    let ret = quote! {
        impl #ident {
            pub fn register_ctors<R: ::tl::parsing::Reader>(cstore: &mut ::tl::dynamic::TLCtorMap<R>) {
                ::tl::Vector::<Box<::tl::dynamic::TLObject>>::register_dynamic(cstore);
                #ident::__register_1(cstore)
            }

            pub fn #ty_count<R: ::tl::parsing::Reader>(_: &mut ::tl::dynamic::TLCtorMap<R>) {}
        }
    };
    ret.parse().unwrap()
}
