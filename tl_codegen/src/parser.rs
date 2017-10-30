use pom::{self, Parser};
use pom::char_class::{alphanum, digit, hex_digit};
use pom::parser::*;

use ast::{Constructor, Delimiter, Field, Item, Type};


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
    (dotted_ident() + tl_id().opt() + fields() - seq(b" = ") + ty_space_generic() - sym(b';'))
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

fn layer() -> Parser<u8, u32> {
    seq(b"// LAYER ") * decimal()
}

fn space() -> Parser<u8, ()> {
    let end_comment = || seq(b"*/");
    ( one_of(b" \n").discard() |
      (seq(b"//") - !(seq(b" LAYER ")) - none_of(b"\n").repeat(0..)).discard() |
      (seq(b"/*") * (!end_comment() * take(1)).repeat(0..) * end_comment()).discard()
    ).repeat(0..).discard()
}

fn item() -> Parser<u8, Item> {
    ( delimiter().map(Item::Delimiter) |
      constructor().map(Item::Constructor) |
      layer().map(Item::Layer)
    ) - space()
}

fn lines() -> Parser<u8, Vec<Item>> {
    space() * item().repeat(0..) - end()
}


pub fn parse_string(input: &str) -> pom::Result<Vec<Item>> {
    let mut input = pom::DataInput::new(input.as_bytes());
    lines().parse(&mut input)
}
