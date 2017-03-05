#![recursion_limit = "128"]

extern crate pom;
#[macro_use]
extern crate quote;
extern crate syn;
extern crate synom;

pub mod parser {
    use pom::char_class::{alphanum, digit, hex_digit};
    use pom::parser::*;
    use pom::{self, Parser};

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
        fn names_vec(&self) -> Option<&Vec<String>> {
            use self::Type::*;
            match self {
                &Int |
                &TypeParameter(..) |
                &Flagged(..) |
                &Repeated(..) => None,
                &Named(ref v) |
                &Generic(ref v, ..) => Some(v),
            }
        }

        pub fn namespace(&self) -> Option<&str> {
            self.names_vec().and_then(|v| {
                match v.len() {
                    1 => None,
                    2 => v.first().map(String::as_str),
                    _ => unimplemented!(),
                }
            })
        }

        pub fn name(&self) -> Option<&str> {
            self.names_vec().and_then(|v| v.last().map(String::as_str))
        }

        pub fn flag_field(&self) -> Option<(&str, u32)> {
            use self::Type::*;
            match self {
                &Flagged(ref f, b, _) => Some((f, b)),
                _ => None,
            }
        }

        pub fn is_type_parameter(&self) -> bool {
            use self::Type::*;
            match self {
                &TypeParameter(..) => true,
                _ => false,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Field {
        pub name: Option<String>,
        pub ty: Type,
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Constructor {
        pub variant: Type,
        pub tl_id: u32,
        pub type_parameters: Vec<Field>,
        pub fields: Vec<Field>,
        pub output: Type,
    }

    #[derive(Debug, Clone)]
    pub enum Delimiter {
        Types,
        Functions,
    }

    #[derive(Debug, Clone)]
    pub enum Item {
        Delimiter(Delimiter),
        Constructor(Constructor),
    }

    fn utf8(v: Vec<u8>) -> String {
        String::from_utf8(v).unwrap()
    }

    fn ident() -> Parser<u8, String> {
        (is_a(alphanum) | sym(b'_')).repeat(1..).map(utf8)
    }

    fn dotted_ident() -> Parser<u8, Vec<String>> {
        ((ident() - sym(b'.')).repeat(0..) + ident())
            .map(|(mut v, i)| {
                v.push(i);
                v
            })
    }

    fn tl_id() -> Parser<u8, u32> {
        sym(b'#') * is_a(hex_digit).repeat(0..9).convert(|s| u32::from_str_radix(&utf8(s), 16))
    }

    fn decimal() -> Parser<u8, u32> {
        is_a(digit).repeat(0..).convert(|s| utf8(s).parse())
    }

    fn ty_flag() -> Parser<u8, Type> {
        (ident() - sym(b'.') + decimal() - sym(b'?') + call(ty))
            .map(|((name, bit), ty)| Type::Flagged(name, bit, Box::new(ty)))
    }

    fn ty_generic() -> Parser<u8, Type> {
        (dotted_ident() - sym(b'<') + call(ty) - sym(b'>'))
            .map(|(name, ty)| Type::Generic(name, Box::new(ty)))
    }

    fn ty() -> Parser<u8, Type> {
        ( sym(b'#').map(|_| Type::Int) |
          sym(b'!') * ident().map(Type::TypeParameter) |
          ty_flag() |
          ty_generic() |
          dotted_ident().map(Type::Named)
        )
    }

    fn ty_space_generic() -> Parser<u8, Type> {
        let space_generic = dotted_ident() - sym(b' ') + ty();
        (space_generic.map(|(name, ty)| Type::Generic(name, Box::new(ty))) |
         ty())
    }

    fn base_field() -> Parser<u8, Field> {
        (ident() - sym(b':') + ty())
            .map(|(name, ty)| Field { name: Some(name), ty: ty })
            .name("field")
    }

    fn repeated_field() -> Parser<u8, Field> {
        sym(b'[')
            * call(base_fields).map(|fv| Field { name: None, ty: Type::Repeated(fv) })
            - seq(b" ]")
    }

    fn base_field_anonymous_or_repeated() -> Parser<u8, Field> {
        ( repeated_field() |
          base_field() |
          ty().map(|ty| Field { name: None, ty: ty }))
    }

    fn base_fields() -> Parser<u8, Vec<Field>> {
        (sym(b' ') * base_field_anonymous_or_repeated()).repeat(0..)
    }

    fn ty_param_field() -> Parser<u8, Field> {
        sym(b'{') * base_field() - sym(b'}')
    }

    fn fields() -> Parser<u8, (Vec<Field>, Vec<Field>)> {
        (sym(b' ') * ty_param_field()).repeat(0..)
            + base_fields()
    }

    fn constructor() -> Parser<u8, Constructor> {
        (dotted_ident() + tl_id() + fields() - seq(b" = ") + ty_space_generic() - sym(b';'))
            .map(|(((variant, tl_id), (type_parameters, fields)), output)| Constructor {
                variant: Type::Named(variant),
                tl_id: tl_id,
                type_parameters: type_parameters,
                fields: fields,
                output: output,
            })
            .name("constructor")
    }

    fn delimiter() -> Parser<u8, Delimiter> {
        ( seq(b"---types---").map(|_| Delimiter::Types) |
          seq(b"---functions---").map(|_| Delimiter::Functions)
        )
    }

    fn space() -> Parser<u8, ()> {
        let end_comment = || seq(b"*/");
        ( one_of(b" \n").discard() |
          (seq(b"//") - none_of(b"\n").repeat(0..)).discard() |
          (seq(b"/*") * (!end_comment() * take(1)).repeat(0..) * end_comment()).discard()
        ).repeat(0..).discard()
    }

    fn item() -> Parser<u8, Item> {
        ( delimiter().map(Item::Delimiter) |
          constructor().map(Item::Constructor)
        ) - space()
    }

    fn lines() -> Parser<u8, Vec<Item>> {
        space() * item().repeat(0..) - end()
    }

    pub fn parse_string(input: &str) -> Result<Vec<Item>, pom::Error> {
        let mut input = pom::DataInput::new(input.as_bytes());
        lines().parse(&mut input)
    }
}

pub use parser::{Constructor, Delimiter, Field, Item, Type};

use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Default)]
struct Constructors(Vec<Constructor>);

#[derive(Debug)]
struct AllConstructors {
    types: BTreeMap<Option<String>, BTreeMap<String, Constructors>>,
    functions: BTreeMap<Option<String>, Vec<Constructor>>,
}

fn filter_items(iv: &mut Vec<Item>) {
    iv.retain(|i| {
        let c = match i {
            &Item::Constructor(ref c) => c,
            _ => return true,
        };
        // Blacklist some annoying inconsistencies.
        match c.variant.name() {
            Some("future_salt") |
            Some("future_salts") |
            Some("vector") => false,
            _ => true,
        }
    })
}

fn partition_by_delimiter_and_namespace(iv: Vec<Item>) -> AllConstructors {
    let mut current = Delimiter::Types;
    let mut ret = AllConstructors {
        types: BTreeMap::new(),
        functions: BTreeMap::new(),
    };
    for i in iv {
        match i {
            Item::Delimiter(d) => current = d,
            Item::Constructor(c) => {
                match current {
                    Delimiter::Types => {
                        ret.types.entry(c.output.namespace().map(Into::into))
                            .or_insert_with(Default::default)
                            .entry(c.output.name().map(Into::into).unwrap())
                            .or_insert_with(Default::default)
                            .0.push(c);
                    },
                    Delimiter::Functions => {
                        ret.functions.entry(c.variant.namespace().map(Into::into))
                            .or_insert_with(Default::default)
                            .push(c);
                    },
                }
            },
        }
    }
    ret
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

fn wrap_option_type(wrap: bool, ty: quote::Tokens) -> quote::Tokens {
    if wrap {
        quote! { Option<#ty> }
    } else {
        ty
    }
}

fn wrap_option_value(wrap: bool, ty: quote::Tokens) -> quote::Tokens {
    if wrap {
        quote! { Some(#ty) }
    } else {
        ty
    }
}

fn all_types_prefix() -> quote::Tokens {
    quote! {::schema::}
}

#[derive(Debug, Clone)]
struct TypeIR {
    tokens: quote::Tokens,
    is_copy: bool,
    needs_box: bool,
    with_option: bool,
}

impl TypeIR {
    fn copyable(tokens: quote::Tokens) -> Self {
        TypeIR {
            tokens: tokens,
            is_copy: true,
            needs_box: false,
            with_option: false,
        }
    }

    fn noncopyable(tokens: quote::Tokens) -> Self {
        TypeIR {
            tokens: tokens,
            is_copy: false,
            needs_box: false,
            with_option: false,
        }
    }

    fn needs_box(tokens: quote::Tokens) -> Self {
        TypeIR {
            tokens: tokens,
            is_copy: false,
            needs_box: true,
            with_option: false,
        }
    }

    fn _unboxed(self) -> quote::Tokens {
        self.tokens
    }

    fn _boxed(self) -> quote::Tokens {
        if self.needs_box {
            let tokens = self.tokens;
            quote! { Box<#tokens> }
        } else {
            self.tokens
        }
    }

    fn _ref_type(self) -> quote::Tokens {
        let needs_ref = self.needs_box || !self.is_copy;
        let ty = self._boxed();
        if needs_ref {
            quote! { &#ty }
        } else {
            ty
        }
    }

    fn unboxed(self) -> quote::Tokens {
        wrap_option_type(self.with_option, self._unboxed())
    }

    fn boxed(self) -> quote::Tokens {
        wrap_option_type(self.with_option, self._boxed())
    }

    fn ref_type(self) -> quote::Tokens {
        wrap_option_type(self.with_option, self._ref_type())
    }

    fn option_wrapped(self) -> Self {
        TypeIR {
            with_option: true,
            ..self
        }
    }

    fn ref_prefix(&self) -> quote::Tokens {
        if self.is_copy {quote!()} else {quote!(ref)}
    }
}

fn names_to_type(names: &Vec<String>) -> TypeIR {
    let type_prefix = all_types_prefix();
    if names.len() == 1 {
        match names[0].as_str() {
            "Bool" => return TypeIR::copyable(quote! { bool }),
            "true" => return TypeIR::copyable(quote! { () }),
            "int" => return TypeIR::copyable(quote! { i32 }),
            "long" => return TypeIR::copyable(quote! { i64 }),
            "int128" => return TypeIR::copyable(quote! { #type_prefix i128 }),
            "int256" => return TypeIR::copyable(quote! { #type_prefix i256 }),
            "double" => return TypeIR::copyable(quote! { f64 }),
            "bytes" => return TypeIR::noncopyable(quote! { Vec<u8> }),
            "string" => return TypeIR::noncopyable(quote! { String }),
            "Vector" => return TypeIR::noncopyable(quote! { Vec }),
            _ => (),
        }
    }
    let mut ty = {
        let name = no_conflict_ident(names[0].as_str());
        quote! {#type_prefix #name}
    };
    if names.len() == 2 {
        let name = no_conflict_ident(names[1].as_str());
        ty = quote! {#ty::#name};
    }
    // Special case two recursive types.
    match names.last().map(String::as_str) {
        Some("PageBlock") |
        Some("RichText") => TypeIR::needs_box(ty),
        _ => TypeIR::noncopyable(ty),
    }
}

impl Type {
    fn as_type(&self) -> TypeIR {
        use Type::*;
        match self {
            &Int => TypeIR::copyable(quote! { i32 }),
            &Named(ref v) => names_to_type(v),
            &TypeParameter(ref s) => {
                let id = no_conflict_ident(s);
                TypeIR::noncopyable(quote! { #id })
            },
            &Generic(ref container, ref ty) => {
                let container = names_to_type(container).unboxed();
                let ty = ty.as_type().unboxed();
                TypeIR::noncopyable(quote! { #container<#ty> })
            },
            &Flagged(_, _, ref ty) => {
                ty.as_type().option_wrapped()
            },
            &Repeated(..) => unimplemented!(),
        }
    }

    fn is_optional(&self) -> bool {
        use Type::*;
        match self {
            &Flagged(..) => true,
            _ => false,
        }
    }
}

impl Field {
    fn as_field(&self) -> quote::Tokens {
        let name = self.name.as_ref().map(|n| no_conflict_ident(n)).unwrap();
        let ty = self.ty.as_type().boxed();

        quote! {
            #name: #ty
        }
    }
}

impl Constructor {
    fn fixup_output(&mut self) {
        if self.is_output_a_type_parameter() {
            self.output = Type::TypeParameter(self.output.name().unwrap().into());
        }
    }

    fn is_output_a_type_parameter(&self) -> bool {
        let output_name = match &self.output {
            &Type::Named(ref v) if v.len() == 1 => v[0].as_str(),
            _ => return false,
        };
        for p in &self.type_parameters {
            if p.name.as_ref().map(String::as_str) == Some(output_name) {
                return true;
            }
        }
        false
    }

    fn flag_field_names(&self) -> HashSet<syn::Ident> {
        let mut ret = HashSet::new();
        for f in &self.fields {
            if let Some((flag, _)) = f.ty.flag_field() {
                ret.insert(no_conflict_ident(flag));
            }
        }
        ret
    }

    fn fields_tokens(&self, pub_: quote::Tokens, trailer: quote::Tokens) -> quote::Tokens {
        let pub_ = std::iter::repeat(pub_);
        if self.fields.is_empty() {
            quote! { #trailer }
        } else {
            let fields = self.fields.iter()
                .map(Field::as_field);
            quote! {
                { #( #pub_ #fields , )* }
            }
        }
    }

    fn as_empty_pattern(&self) -> quote::Tokens {
        let name = self.variant_name();
        if self.fields.is_empty() {
            quote! { #name }
        } else {
            quote! { #name {..} }
        }
    }

    fn generics(&self) -> quote::Tokens {
        if self.type_parameters.is_empty() {
            return quote!();
        }
        let tys = self.type_parameters.iter()
            .map(|f| no_conflict_ident(f.name.as_ref().unwrap()));
        quote! { <#(#tys),*> }
    }

    fn rpc_generics(&self) -> quote::Tokens {
        if self.type_parameters.is_empty() {
            return quote!();
        }
        let tys = self.type_parameters.iter()
            .map(|f| no_conflict_ident(f.name.as_ref().unwrap()));
        let traits = std::iter::repeat(quote!(::rpc::RpcFunction));
        quote! { <#(#tys: #traits),*> }
    }

    fn type_generics(&self) -> quote::Tokens {
        if self.type_parameters.is_empty() {
            return quote!();
        }
        let tys = self.type_parameters.iter()
            .map(|f| no_conflict_ident(f.name.as_ref().unwrap()));
        let traits = std::iter::repeat(quote!(::tl::Type));
        quote! { <#(#tys: #traits),*> }
    }

    fn as_struct_update_flags_method(&self) -> Option<quote::Tokens> {
        let flag_fields = self.flag_field_names();
        if flag_fields.is_empty() {
            return None;
        }
        let fields = self.fields.iter()
            .filter_map(|f| {
                let name = no_conflict_ident(f.name.as_ref().unwrap());
                f.ty.flag_field().map(|(flag_field, bit)| {
                    let flag_field = no_conflict_ident(flag_field);
                    quote! {
                        if self.#name.is_some() {
                            self.#flag_field |= 1 << #bit;
                        }
                    }
                })
            });

        Some(quote! {
            pub fn update_flags(&mut self) {
                #( #fields )*
            }

            pub fn clear_and_update_flags(&mut self) {
                #( self.#flag_fields = 0; )*
                self.update_flags();
            }
        })
    }

    fn as_struct_base(&self, name: &syn::Ident) -> quote::Tokens {
        let generics = self.generics();
        let impl_block = self.as_struct_update_flags_method()
            .map(|b| quote! {
                impl #generics #name #generics {
                    #b
                }
            })
            .unwrap_or_else(|| quote!());
        let fields = self.fields_tokens(quote! {pub}, quote! {;});
        quote! {
            #[derive(Debug, Clone)]
            pub struct #name #generics #fields #impl_block
        }
    }

    fn as_struct_deserialize(&self) -> (quote::Tokens, quote::Tokens) {
        if self.fields.is_empty() {
            return (quote!(), quote!());
        }
        let flag_fields = self.flag_field_names();
        let constructor = {
            let fields = self.fields.iter()
                .map(|f| {
                    let name = no_conflict_ident(f.name.as_ref().unwrap());
                    if flag_fields.contains(&name) {
                        quote! { #name: { #name = _reader.read_generic()?; #name } }
                    } else if let Some((flag_field, bit)) = f.ty.flag_field() {
                        let flag_field = no_conflict_ident(flag_field);
                        quote! {
                            #name: if #flag_field & (1 << #bit) == 0 {
                                None
                            } else {
                                Some(_reader.read_generic()?)
                            }
                        }
                    } else {
                        quote! { #name: _reader.read_generic()? }
                    }
                });
            quote!({ #( #fields, )* })
        };
        let flag_lets = quote! {
            #( let #flag_fields; )*
        };
        (flag_lets, constructor)
    }

    fn as_variant_update_flags(&self, name: &syn::Ident) -> Option<(quote::Tokens, quote::Tokens)> {
        let flag_fields_ = self.flag_field_names();
        if flag_fields_.is_empty() {
            return None;
        }
        let (field_names, field_tests): (Vec<_>, Vec<_>) = self.fields.iter()
            .filter_map(|f| {
                let name = no_conflict_ident(f.name.as_ref().unwrap());
                f.ty.flag_field().map(|(flag_field, bit)| {
                    let flag_field = no_conflict_ident(flag_field);
                    (quote!(#name), quote! {
                        if #name.is_some() {
                            *#flag_field |= 1 << #bit;
                        }
                    })
                })
            })
            .unzip();

        let flag_fields = &flag_fields_;

        Some((quote! {
            #name { #( ref mut #flag_fields ),* , #( ref #field_names ),* , .. } => {
                #( #field_tests )*
            }
        }, quote! {
            #name { #( ref mut #flag_fields ),* , .. } => {
                #( *#flag_fields = 0; )*
            }
        }))
    }

    fn as_struct(&self) -> quote::Tokens {
        let name = self.output.name().map(|n| no_conflict_ident(n)).unwrap();
        let tl_id = self.tl_id();
        let serialize_destructure = self.as_variant_ref_destructure(&name)
            .map(|d| quote! { let &#d = self; })
            .unwrap_or_else(|| quote!());
        let serialize_stmts = self.as_variant_serialize();
        let (flag_lets, deserialize) = self.as_struct_deserialize();
        let type_impl = self.as_type_impl(
            &name,
            quote!(Some(#tl_id)),
            quote!(#serialize_destructure #serialize_stmts Ok(())),
            quote!(Ok({ #flag_lets #name #deserialize })),
            quote! {
                if _id == #tl_id {
                    Self::deserialize(_reader)
                } else {
                    Err(::error::ErrorKind::InvalidType(_id).into())
                }
            });
        let struct_block = self.as_struct_base(&name);
        quote! {
            #struct_block
            #type_impl
        }
    }

    fn variant_name(&self) -> syn::Ident {
        self.variant.name().map(|n| no_conflict_ident(n)).unwrap()
    }

    fn as_variant_ref_destructure(&self, name: &syn::Ident) -> Option<quote::Tokens> {
        if self.fields.is_empty() {
            return None;
        }
        let fields = self.fields.iter()
            .map(|f| {
                let name = no_conflict_ident(f.name.as_ref().unwrap());
                quote! { ref #name }
            });
        Some(quote! {
            #name { #( #fields ),* }
        })
    }

    fn as_variant_serialize(&self) -> quote::Tokens {
        let fields = self.fields.iter()
            .map(|f| {
                let name = no_conflict_ident(f.name.as_ref().unwrap());
                quote! { _writer.write_generic(#name)?; }
            });
        quote! {
            #( #fields )*
        }
    }

    fn as_function_struct(&self) -> quote::Tokens {
        let name = self.variant_name();
        let rpc_generics = self.rpc_generics();
        let generics = self.generics();
        let struct_block = self.as_struct_base(&name);
        let mut output_ty = self.output.as_type().unboxed();
        if self.output.is_type_parameter() {
            output_ty = quote! {#output_ty::Reply};
        }
        let tl_id = self.tl_id();
        let serialize_destructure = self.as_variant_ref_destructure(&name)
            .map(|d| quote! { let &#d = self; })
            .unwrap_or_else(|| quote!());
        let serialize_stmts = self.as_variant_serialize();
        let type_impl = self.as_type_impl(
            &name,
            quote!(Some(#tl_id)),
            quote!(#serialize_destructure #serialize_stmts Ok(())),
            quote!(Err(::error::ErrorKind::ReceivedSendType.into())),
            quote!(Err(::error::ErrorKind::ReceivedSendType.into())));
        quote! {
            #struct_block
            impl #rpc_generics ::rpc::RpcFunction for #name #generics {
                type Reply = #output_ty;
            }
            #type_impl
        }
    }

    fn as_variant(&self) -> quote::Tokens {
        let name = self.variant_name();
        let fields = self.fields_tokens(quote! {}, quote! {});
        quote! { #name #fields }
    }

    fn tl_id(&self) -> quote::Tokens {
        let tl_id: syn::Ident = format!("0x{:08x}", self.tl_id).into();
        quote! { ::tl::parsing::ConstructorId(#tl_id) }
    }

    fn as_type_impl(&self, name: &syn::Ident, type_id: quote::Tokens, serialize: quote::Tokens, deserialize: quote::Tokens, deserialize_boxed: quote::Tokens) -> quote::Tokens {
        let type_generics = self.type_generics();
        let generics = self.generics();
        quote! {

            impl #type_generics ::tl::Type for #name #generics {
                #[inline]
                fn bare_type() -> bool {
                    false
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
        }
    }
}

impl Constructors {
    fn coalesce_methods(&self) -> BTreeMap<&str, BTreeMap<&Type, BTreeSet<&Constructor>>> {
        let mut map: BTreeMap<&str, BTreeMap<&Type, BTreeSet<&Constructor>>> = BTreeMap::new();
        for cons in &self.0 {
            for field in &cons.fields {
                let name = match field.name.as_ref() {
                    Some(s) => s.as_str(),
                    None => continue,
                };
                map.entry(name)
                    .or_insert_with(Default::default)
                    .entry(&field.ty)
                    .or_insert_with(Default::default)
                    .insert(cons);
            }
        }
        map
    }

    fn as_update_flags_method(&self, enum_name: &syn::Ident) -> Option<quote::Tokens> {
        let mut any_flags = false;
        let (flag_sets, flag_clears): (Vec<_>, Vec<_>) = self.0.iter()
            .map(|c| {
                let variant_name = c.variant_name();
                c.as_variant_update_flags(&variant_name)
                    .map(|(flag_set, flag_clear)| {
                        any_flags = true;
                        (quote!(&mut #enum_name :: #flag_set),
                         quote!(&mut #enum_name :: #flag_clear))
                    })
                    .unwrap_or_else(|| {
                        let pat = c.as_empty_pattern();
                        let empty = quote! { &mut #enum_name :: #pat => () };
                        (empty.clone(), empty)
                    })
            })
            .unzip();

        if !any_flags {
            return None;
        }

        Some(quote! {
            pub fn update_flags(&mut self) {
                match self {
                    #( #flag_sets, )*
                }
            }

            pub fn clear_and_update_flags(&mut self) {
                match self {
                    #( #flag_clears, )*
                }
                self.update_flags();
            }
        })
    }

    fn determine_methods(&self, enum_name: &syn::Ident) -> quote::Tokens {
        let all_constructors = self.0.len();
        let mut methods = vec![];
        for (name, typemap) in self.coalesce_methods() {
            if typemap.len() != 1 {
                continue;
            }
            let (output_ty, constructors) = typemap.into_iter().next().unwrap();
            if constructors.len() <= 1 {
                continue;
            }
            let name = no_conflict_ident(name);
            let mut ty_ir = output_ty.as_type();
            if constructors.len() != all_constructors {
                ty_ir.with_option = true;
            }
            let value = wrap_option_value(ty_ir.with_option, quote!(#name));
            let ref_ = ty_ir.ref_prefix();
            let field = if output_ty.is_optional() {
                quote! { #name: Some(#ref_ #name) }
            } else {
                quote! { #ref_ #name }
            };
            let constructors = constructors.into_iter()
                .map(|c| {
                    let cons_name = c.variant_name();
                    quote! { & #enum_name :: #cons_name { #field, .. } => #value, }
                });
            let trailer = if !ty_ir.with_option {
                quote! {}
            } else {
                quote! { _ => None, }
            };
            let ty = ty_ir.ref_type();
            methods.push(quote! {
                pub fn #name(&self) -> #ty {
                    match self {
                        #( #constructors )*
                        #trailer
                    }
                }
            });
        }

        methods.extend(self.as_update_flags_method(enum_name));

        if methods.is_empty() {
            quote! {}
        } else {
            quote! {
                impl #enum_name {
                    #( #methods )*
                }
            }
        }
    }

    fn as_type_impl(&self, name: &syn::Ident, type_id: quote::Tokens, serialize: quote::Tokens, deserialize: quote::Tokens, deserialize_boxed: quote::Tokens) -> quote::Tokens {
        quote! {

            impl ::tl::Type for #name {
                #[inline]
                fn bare_type() -> bool {
                    false
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
        }
    }

    fn as_type_id_match(&self, enum_name: &syn::Ident) -> quote::Tokens {
        let constructors = self.0.iter()
            .map(|c| {
                let pat = c.as_empty_pattern();
                let tl_id = c.tl_id();
                quote! { & #enum_name :: #pat => Some(#tl_id) }
            });
        quote! {
            match self {
                #( #constructors, )*
            }
        }
    }

    fn as_serialize_match(&self, enum_name: &syn::Ident) -> quote::Tokens {
        let constructors = self.0.iter()
            .map(|c| {
                let variant_name = c.variant_name();
                let serialize_destructure = c.as_variant_ref_destructure(&variant_name)
                    .unwrap_or_else(|| quote!(#variant_name));
                let serialize_stmts = c.as_variant_serialize();
                quote! { & #enum_name :: #serialize_destructure => { #serialize_stmts } }
            });
        quote! {
            match self {
                #( #constructors, )*
            }
            Ok(())
        }
    }

    fn as_deserialize_match(&self, enum_name: &syn::Ident) -> quote::Tokens {
        let constructors = self.0.iter()
            .map(|c| {
                let variant_name = c.variant_name();
                let tl_id = c.tl_id();
                let (flag_lets, deserialize) = c.as_struct_deserialize();
                quote! { #tl_id => { #flag_lets Ok(#enum_name :: #variant_name #deserialize) } }
            });
        quote! {
            match _id {
                #( #constructors, )*
                _ => Err(::error::ErrorKind::InvalidType(_id).into()),
            }
        }
    }

    fn as_struct(&self) -> quote::Tokens {
        if self.0.len() == 1 {
            return self.0[0].as_struct();
        }

        let name = self.0[0].output.name().map(|n| no_conflict_ident(n)).unwrap();
        let variants = self.0.iter()
            .map(Constructor::as_variant);
        let methods = self.determine_methods(&name);
        let type_impl = self.as_type_impl(
            &name,
            self.as_type_id_match(&name),
            self.as_serialize_match(&name),
            quote!(Err(::error::ErrorKind::BoxedAsBare.into())),
            self.as_deserialize_match(&name));

        quote! {
            #[derive(Debug, Clone)]
            pub enum #name {
                #( #variants , )*
            }
            #methods
            #type_impl
        }
    }
}

pub fn generate_code_for(input: &str) -> String {
    let constructors = {
        let mut items = parser::parse_string(input).unwrap();
        filter_items(&mut items);
        partition_by_delimiter_and_namespace(items)
    };

    let mut items = vec![quote! {
        #![allow(non_camel_case_types)]
        pub type i128 = (i64, i64);
        pub type i256 = (i128, i128);
        use rpc::functions::FutureSalts;
    }];

    for (ns, constructor_map) in constructors.types {
        let substructs = constructor_map.values()
            .map(Constructors::as_struct);
        match ns {
            None => items.extend(substructs),
            Some(name) => {
                let name = no_conflict_ident(name.as_str());
                items.push(quote! {
                    pub mod #name {
                        #( #substructs )*
                    }
                });
            },
        }
    }

    let mut rpc_items = vec![];
    for (ns, mut substructs) in constructors.functions {
        substructs.sort_by_key(|c| c.variant.clone());
        let substructs = substructs.into_iter()
            .map(|mut c| {
                c.fixup_output();
                c.as_function_struct()
            });
        match ns {
            None => rpc_items.extend(substructs),
            Some(name) => {
                let name = no_conflict_ident(name.as_str());
                rpc_items.push(quote! {
                    pub mod #name {
                        #( #substructs )*
                    }
                });
            },
        }
    }
    items.push(quote! {
        pub mod rpc {
            #( #rpc_items )*
        }
    });

    (quote! { #(#items)* }).to_string()
}
