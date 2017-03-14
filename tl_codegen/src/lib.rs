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
        pub fn names_vec(&self) -> Option<&Vec<String>> {
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
use std::mem;

#[derive(Debug, Default)]
struct Constructors(Vec<Constructor>);

#[derive(Debug)]
struct AllConstructors {
    types: BTreeMap<Option<String>, BTreeMap<String, Constructors>>,
    functions: BTreeMap<Option<String>, Vec<Constructor>>,
}

fn filter_items(iv: &mut Vec<Item>) {
    let mut ensure_types = vec![Item::Delimiter(Delimiter::Types)];
    iv.retain(|i| {
        let c = match i {
            &Item::Constructor(ref c) => c,
            _ => return true,
        };
        // Blacklist some annoying inconsistencies.
        match c.variant.name() {
            Some("true") |
            Some("vector") => false,
            Some("future_salt") |
            Some("future_salts") => {
                ensure_types.push(Item::Constructor(c.clone()));
                false
            },
            _ => true,
        }
    });
    iv.extend(ensure_types);
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
            "int128" => return TypeIR::copyable(quote! { #type_prefix Int128 }),
            "int256" => return TypeIR::copyable(quote! { #type_prefix Int256 }),
            "double" => return TypeIR::copyable(quote! { f64 }),
            "bytes" => return TypeIR::noncopyable(quote! { Vec<u8> }),
            "string" => return TypeIR::noncopyable(quote! { String }),
            "vector" => return TypeIR::noncopyable(quote! { #type_prefix BareVec }),
            "Vector" => return TypeIR::noncopyable(quote! { Vec }),
            "future_salt" => return TypeIR::noncopyable(quote! { #type_prefix FutureSalt }),
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
    fn fixup(&mut self, which: Delimiter) {
        if which == Delimiter::Functions {
            self.fixup_output();
        }
        self.fixup_fields();
    }

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

    fn fixup_fields(&mut self) {
        match self.variant.name() {
            Some("resPQ") |
            Some("p_q_inner_data") |
            Some("server_DH_params_ok") |
            Some("server_DH_inner_data") |
            Some("client_DH_inner_data") |
            Some("req_DH_params") |
            Some("set_client_DH_params") => (),
            _ => return,
        }
        for f in &mut self.fields {
            match &mut f.ty {
                &mut Type::Named(ref mut v) if v.len() == 1 && v[0] == "string" => {
                    v[0] = "bytes".into();
                },
                _ => (),
            }
        }
    }

    fn flag_field_names(&self) -> HashSet<&str> {
        let mut ret = HashSet::new();
        for f in &self.fields {
            if let Some((flag, _)) = f.ty.flag_field() {
                ret.insert(flag);
            }
        }
        ret
    }

    fn flag_field_idents(&self) -> HashSet<syn::Ident> {
        self.flag_field_names()
            .into_iter()
            .map(no_conflict_ident)
            .collect()
    }

    fn non_flag_fields<'a>(&'a self) -> Box<Iterator<Item = &'a Field> + 'a> {
        let flag_fields = self.flag_field_names();
        Box::new({
            self.fields.iter()
                .filter(move |f| {
                    f.name.as_ref()
                        .map(|s| !flag_fields.contains(s.as_str()))
                        .unwrap_or(true)
                })
        })
    }

    fn fields_tokens(&self, pub_: quote::Tokens, trailer: quote::Tokens) -> quote::Tokens {
        let pub_ = std::iter::repeat(pub_);
        if self.fields.is_empty() {
            quote! { #trailer }
        } else {
            let fields = self.non_flag_fields()
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

    fn type_generics(&self, trait_: &quote::Tokens) -> quote::Tokens {
        if self.type_parameters.is_empty() {
            return quote!();
        }
        let tys = self.type_parameters.iter()
            .map(|f| no_conflict_ident(f.name.as_ref().unwrap()));
        let traits = std::iter::repeat(trait_);
        quote! { <#(#tys: #traits),*> }
    }

    fn as_struct_determine_flags(&self, field_prefix: quote::Tokens) -> Option<(HashSet<syn::Ident>, quote::Tokens)> {
        let flag_fields = self.flag_field_idents();
        if flag_fields.is_empty() {
            return None;
        }
        let determination = {
            let fields = self.fields.iter()
                .filter_map(|f| {
                    let name = no_conflict_ident(f.name.as_ref().unwrap());
                    f.ty.flag_field().map(|(flag_field, bit)| {
                        let flag_field = no_conflict_ident(flag_field);
                        quote! {
                            if #field_prefix #name.is_some() {
                                #flag_field |= 1 << #bit;
                            }
                        }
                    })
                });
            let flag_fields = &flag_fields;
            quote! {
                #( let mut #flag_fields = 0i32; )*
                #( #fields )*
            }
        };
        Some((flag_fields, determination))
    }

    fn as_struct_base(&self, name: &syn::Ident) -> quote::Tokens {
        let generics = self.generics();
        let fields = self.fields_tokens(quote! {pub}, quote! {;});
        quote! {
            #[derive(Debug, Clone)]
            pub struct #name #generics #fields
        }
    }

    fn as_struct_deserialize(&self) -> (HashSet<syn::Ident>, quote::Tokens) {
        if self.fields.is_empty() {
            return (HashSet::new(), quote!());
        }
        let flag_fields = self.flag_field_idents();
        let mut flags_to_read = vec![];
        let constructor = {
            let fields = self.fields.iter()
                .filter_map(|f| {
                    let name = no_conflict_ident(f.name.as_ref().unwrap());
                    let mut expr = if flag_fields.contains(&name) {
                        flags_to_read.push(name);
                        return None;
                    } else if let Some((flag_field, bit)) = f.ty.flag_field() {
                        let flag_field = no_conflict_ident(flag_field);
                        quote! {
                            if #flag_field & (1 << #bit) == 0 {
                                None
                            } else {
                                Some(_reader.read_tl()?)
                            }
                        }
                    } else {
                        quote!(_reader.read_tl()?)
                    };
                    if !flags_to_read.is_empty() {
                        let flags = mem::replace(&mut flags_to_read, vec![]);
                        expr = quote!({
                            #( #flags = _reader.read_tl()?; )*
                            #expr
                        })
                    }
                    Some(quote!(#name: #expr))
                });
            quote!({ #( #fields, )* })
        };
        (flag_fields, constructor)
    }

    fn as_struct(&self) -> quote::Tokens {
        let name = self.output.name().map(|n| no_conflict_ident(n)).unwrap();
        let tl_id = self.tl_id();
        let serialize_destructure = self.as_variant_ref_destructure(&name)
            .map(|d| quote! { let &#d = self; })
            .unwrap_or_else(|| quote!());
        let serialize_stmts = self.as_variant_serialize();
        let (flag_fields_, deserialize) = self.as_struct_deserialize();
        let flag_fields = &flag_fields_;
        let type_impl = self.as_type_impl(
            &name,
            quote!(Some(#tl_id)),
            quote!(#serialize_destructure #serialize_stmts Ok(())),
            Some(quote!(match _id {
                Some(#tl_id) | None => Ok({
                    #( let #flag_fields: i32; )*
                    #name #deserialize
                }),
                id => Err(::error::ErrorKind::InvalidType(vec![#tl_id], id).into()),
            })));
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
        let fields = self.non_flag_fields()
            .map(|f| {
                let name = no_conflict_ident(f.name.as_ref().unwrap());
                quote! { ref #name }
            });
        Some(quote! {
            #name { #( #fields ),* }
        })
    }

    fn as_variant_serialize(&self) -> quote::Tokens {
        let (flag_fields, determine_flags) = self.as_struct_determine_flags(quote!())
            .unwrap_or_else(|| (HashSet::new(), quote!()));
        let fields = self.fields.iter()
            .map(|f| {
                let name = no_conflict_ident(f.name.as_ref().unwrap());
                if flag_fields.contains(&name) {
                    quote! { _writer.write_tl(&#name)?; }
                } else if f.ty.flag_field().is_some() {
                    quote! {
                        if let &Some(ref inner) = #name {
                            _writer.write_tl(inner)?;
                        }
                    }
                } else {
                    quote! { _writer.write_tl(#name)?; }
                }
            });
        quote! {
            #determine_flags
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
            None);
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

    fn as_type_impl(&self, name: &syn::Ident, type_id: quote::Tokens, serialize: quote::Tokens, deserialize: Option<quote::Tokens>) -> quote::Tokens {
        let write_generics = self.type_generics(&quote!(::tl::WriteType));
        let generics = self.generics();

        let deserialize = deserialize.map(|body| {
            let read_generics = self.type_generics(&quote!(::tl::ReadType));
            quote! {
                impl #read_generics ::tl::ReadType for #name #generics {
                    fn deserialize_bare<R: ::tl::parsing::Reader>(
                        _id: Option<::tl::parsing::ConstructorId>,
                        _reader: &mut R
                    ) -> ::tl::Result<Self> {
                        #body
                    }
                }
            }
        }).unwrap_or_else(|| quote!());

        quote! {

            impl #generics ::tl::IdentifiableType for #name #generics {
                fn type_id(&self) -> Option<::tl::parsing::ConstructorId> {
                    #type_id
                }
            }

            impl #write_generics ::tl::WriteType for #name #generics {
                fn serialize<W: ::tl::parsing::Writer>(
                    &self,
                    _writer: &mut W
                ) -> ::tl::Result<()> {
                    #serialize
                }
            }

            #deserialize
        }
    }
}

impl Constructors {
    fn fixup(&mut self, delim: Delimiter) {
        for c in &mut self.0 {
            c.fixup(delim);
        }
    }

    fn coalesce_methods(&self) -> BTreeMap<&str, BTreeMap<&Type, BTreeSet<&Constructor>>> {
        let mut map: BTreeMap<&str, BTreeMap<&Type, BTreeSet<&Constructor>>> = BTreeMap::new();
        for cons in &self.0 {
            for field in cons.non_flag_fields() {
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

    fn as_type_impl(&self, name: &syn::Ident, type_id: quote::Tokens, serialize: quote::Tokens, deserialize: quote::Tokens) -> quote::Tokens {
        quote! {

            impl ::tl::IdentifiableType for #name {
                fn type_id(&self) -> Option<::tl::parsing::ConstructorId> {
                    #type_id
                }
            }

            impl ::tl::WriteType for #name {
                fn serialize<W: ::tl::parsing::Writer>(
                    &self,
                    _writer: &mut W
                ) -> ::tl::Result<()> {
                    #serialize
                }
            }

            impl ::tl::ReadType for #name {
                fn deserialize_bare<R: ::tl::parsing::Reader>(
                    _id: Option<::tl::parsing::ConstructorId>,
                    _reader: &mut R
                ) -> ::tl::Result<Self> {
                    #deserialize
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
        let tl_ids = self.0.iter()
            .map(|c| c.tl_id());
        let constructors = self.0.iter()
            .map(|c| {
                let variant_name = c.variant_name();
                let tl_id = c.tl_id();
                let (flag_fields, deserialize) = c.as_struct_deserialize();
                quote!(Some(#tl_id) => {
                    #( let #flag_fields: i32; )*
                    Ok(#enum_name :: #variant_name #deserialize)
                })
            });
        quote! {
            match _id {
                #( #constructors, )*
                id => Err(::error::ErrorKind::InvalidType(vec![#( #tl_ids ),*], id).into()),
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

    fn as_dynamic_ctors(&self) -> Vec<(Option<Vec<String>>, u32, quote::Tokens)> {
        let ty = self.0[0].output.as_type().unboxed();
        let ty_name = self.0[0].output.names_vec();
        self.0.iter()
            .map(|c| {
                let tl_id = c.tl_id();
                (ty_name.cloned(), c.tl_id, quote!(cstore.add::<#ty>(#tl_id)))
            })
            .collect()
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
        pub use manual_types::*;
    }];

    let mut dynamic_ctors = vec![];
    for (ns, mut constructor_map) in constructors.types {
        dynamic_ctors.extend(
            constructor_map.values().flat_map(Constructors::as_dynamic_ctors));
        let substructs = constructor_map.values_mut()
            .map(|mut c| {
                c.fixup(Delimiter::Functions);
                c.as_struct()
            });
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

    dynamic_ctors.sort_by(|t1, t2| (&t1.0, t1.1).cmp(&(&t2.0, t2.1)));
    let dynamic_ctors = dynamic_ctors.into_iter()
        .map(|t| t.2);
    items.push(quote! {
        pub fn register_ctors<R: ::tl::parsing::Reader>(cstore: &mut ::tl::dynamic::TLCtorMap<R>) {
            register_manual_ctors(cstore);
            #( #dynamic_ctors; )*
        }
    });

    let mut rpc_items = vec![];
    for (ns, mut substructs) in constructors.functions {
        substructs.sort_by(|c1, c2| c1.variant.cmp(&c2.variant));
        let substructs = substructs.into_iter()
            .map(|mut c| {
                c.fixup(Delimiter::Functions);
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
