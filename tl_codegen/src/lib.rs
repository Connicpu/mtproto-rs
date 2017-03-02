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

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
struct Constructors(Vec<Constructor>);

#[derive(Debug)]
struct AllConstructors {
    types: BTreeMap<Option<String>, BTreeMap<String, Constructors>>,
    functions: BTreeMap<Option<String>, Vec<Constructor>>,
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

fn names_to_type(names: &Vec<String>, prefix: quote::Tokens) -> (bool, quote::Tokens) {
    let type_prefix = all_types_prefix();
    if names.len() == 1 {
        match names[0].as_str() {
            "Bool" => return (true, quote! { bool }),
            "true" => return (true, quote! { () }),
            "int" => return (true, quote! { i32 }),
            "long" => return (true, quote! { i64 }),
            "int128" => return (true, quote! { #type_prefix i128 }),
            "int256" => return (true, quote! { #type_prefix i256 }),
            "double" => return (true, quote! { f64 }),
            "bytes" => return (false, quote! { #prefix Vec<u8> }),
            "string" => return (false, quote! { #prefix String }),
            "vector" => return (false, quote! { #prefix BareVector }),
            "Vector" => return (false, quote! { #prefix Vec }),
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
    (false, ty)
}

impl Type {
    fn into_type_base(&self, prefix: quote::Tokens) -> (bool, quote::Tokens) {
        use Type::*;
        match self {
            &Int => (true, quote! { i32 }),
            &Named(ref v) => names_to_type(v, prefix),
            &TypeParameter(ref s) => {
                let id = no_conflict_ident(s);
                (false, quote! { #prefix #id })
            },
            &Generic(ref container, ref ty) => {
                let (_, container) = names_to_type(container, prefix);
                let (_, ty) = ty.into_type();
                (false, quote! { #container<#ty> })
            },
            &Flagged(_, _, ref ty) => {
                let (is_copy, ty) = ty.into_type_base(prefix);
                (is_copy, quote! { Option<#ty> })
            },
            &Repeated(..) => unimplemented!(),
        }
    }

    fn into_type(&self) -> (bool, quote::Tokens) {
        self.into_type_base(quote! {})
    }

    fn into_ref_type(&self) -> (bool, quote::Tokens) {
        self.into_type_base(quote! {&})
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
    fn into_field(&self) -> quote::Tokens {
        let name = self.name.as_ref().map(|n| no_conflict_ident(n)).unwrap();
        let (_, ty) = self.ty.into_type();

        quote! {
            #name: #ty
        }
    }
}

impl Constructor {
    fn fields_tokens(&self, pub_: quote::Tokens, trailer: quote::Tokens) -> quote::Tokens {
        let pub_ = std::iter::repeat(pub_);
        if self.fields.is_empty() {
            quote! { #trailer }
        } else {
            let fields = self.fields.iter()
                .map(Field::into_field);
            quote! {
                { #( #pub_ #fields , )* }
            }
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

    fn into_struct_base(&self, name: syn::Ident) -> quote::Tokens {
        let generics = self.generics();
        let fields = self.fields_tokens(quote! {pub}, quote! {;});
        quote! {
            #[derive(Debug, Clone)]
            pub struct #name #generics #fields
        }
    }

    fn into_struct(&self) -> quote::Tokens {
        let name = self.output.name().map(|n| no_conflict_ident(n)).unwrap();
        self.into_struct_base(name)
    }

    fn variant_name(&self) -> syn::Ident {
        self.variant.name().map(|n| no_conflict_ident(n)).unwrap()
    }

    fn into_function_struct(&self) -> quote::Tokens {
        self.into_struct_base(self.variant_name())
    }

    fn into_variant(&self) -> quote::Tokens {
        let name = self.variant_name();
        let fields = self.fields_tokens(quote! {}, quote! {});
        quote! { #name #fields }
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
            let (is_copy, ty) = output_ty.into_ref_type();
            let ty = wrap_option_type(
                !output_ty.is_optional() && constructors.len() != all_constructors, ty);
            let is_option_type = output_ty.is_optional() || constructors.len() != all_constructors;
            let value = wrap_option_value(is_option_type, quote!(#name));
            let ref_ = if is_copy {quote!()} else {quote!(ref)};
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
            let trailer = if !is_option_type {
                quote! {}
            } else {
                quote! { _ => None, }
            };
            methods.push(quote! {
                pub fn #name(&self) -> #ty {
                    match self {
                        #( #constructors )*
                        #trailer
                    }
                }
            });
        }
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

    fn into_struct(&self) -> quote::Tokens {
        if self.0.len() == 1 {
            return self.0[0].into_struct();
        }


        let name = self.0[0].output.name().map(|n| no_conflict_ident(n)).unwrap();
        let methods = self.determine_methods(&name);
        let variants = self.0.iter()
            .map(Constructor::into_variant);
        quote! {

            #[derive(Debug, Clone)]
            pub enum #name {
                #( #variants , )*
            }

            #methods

        }
    }
}

pub fn generate_code_for(input: &str) -> String {
    let constructors = partition_by_delimiter_and_namespace(
        parser::parse_string(input).unwrap());

    let mut items = vec![quote! {
        pub type i128 = (i64, i64);
        pub type i256 = (i128, i128);
    }];

    for (ns, constructor_map) in constructors.types {
        let substructs = constructor_map.values()
            .map(Constructors::into_struct);
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
            .map(|c| c.into_function_struct());
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
