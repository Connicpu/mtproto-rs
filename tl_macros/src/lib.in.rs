extern crate syntex;
extern crate syntex_syntax as syntax;
extern crate aster;
extern crate quasi;

use syntex::Registry;
use syntax::ast::{self, Item, MetaItem};
use syntax::codemap::Span;
use syntax::ext::base::{ExtCtxt, MultiItemDecorator, Annotatable};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

pub struct TLIdExpander;
impl MultiItemDecorator for TLIdExpander {
    fn expand(&self, cx: &mut ExtCtxt, span: Span, meta_item: &MetaItem, item: &Annotatable, push: &mut FnMut(Annotatable)) {
        let item = match *item {
            Annotatable::Item(ref item) => item,
            _ => return,
        };
        
        let builder = aster::AstBuilder::new().span(span);
        let ty = builder.ty().path().segment(item.ident).build().build();
        
        match item.node {
            ast::ItemStruct(_, _) => {},
            _ => return,
        }
        
        let meta_list = match meta_item.node {
            ast::MetaList(_, ref list) => list,
            _ => return,
        };
        
        if meta_list.len() != 1 {
            cx.span_err(span, "#[tl_id(...)] takes exactly 1 parameter");
            return;
        }
        
        let tl_id = match meta_list[0].node {
            ast::MetaWord(ref tl_id) => &**tl_id,
            _ => {
                cx.span_err(span, "#[tl_id(...)] takes the hex id of the type prefixed by an underscore");
                return;
            }
        };
        
        let id = match u32::from_str_radix(&tl_id[1..], 16) {
            Ok(id) => id,
            Err(_) => {
                cx.span_err(span, "#[tl_id(...)] takes the hex id of the type prefixed by an underscore");
                return;
            }
        };
        
        let item = quote_item!(cx,
            impl $ty {
                const TYPE_ID: ::tl::parsing::ConstructorId = ::tl::parsing::ConstructorId($id);
            }
        ).unwrap();
        
        push(Annotatable::Item(item));
    }
}

pub struct ComplexExpander;
impl MultiItemDecorator for ComplexExpander {
    fn expand(&self, cx: &mut ExtCtxt, span: Span, meta_item: &MetaItem, item: &Annotatable, push: &mut FnMut(Annotatable)) {
        let item = match *item {
            Annotatable::Item(ref item) => item,
            _ => {
                cx.span_err(
                    meta_item.span,
                    "`derive(TLType)` may only be applied to structs and enums"
                );
                return;
            }
        };
        
        let builder = aster::AstBuilder::new().span(span);
        let ty = builder.ty().path().segment(item.ident).build().build();
        
        let body = impl_body(
            cx,
            &builder,
            &item,
            ty
        );
        
        push(Annotatable::Item(body));
    }
}

fn impl_body(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
    ty: P<ast::Ty>
) -> P<ast::Item> {
    let (type_id, serialize, deserialize, deserialize_box) = match item.node {
        ast::ItemStruct(ref variant_data, _) => {
            impl_item_struct(
                cx,
                builder,
                item,
                variant_data
            )
        }
        ast::ItemEnum(ref enum_def, _) => {
            impl_item_enum(
                cx,
                builder,
                item.ident,
                enum_def
            )
        }
        _ => cx.bug("expected ItemStruct or ItemEnum in #[derive(TLType)]")
    };
    
    
    quote_item!(
        cx,
        #[allow(unused_variables)]
        impl ::tl::Type for $ty {
            fn bare_type() -> bool {
                false
            }
            
            fn type_id(&self) -> Option<::tl::parsing::ConstructorId> {
                $type_id
            }
            
            fn serialize<W: ::std::io::Write>(
                &self,
                writer: &mut ::tl::parsing::WriteContext<W>
            ) -> ::tl::Result<()> {
                $serialize
            }
            
            fn deserialize<R: ::std::io::Read>(
                reader: &mut ::tl::parsing::ReadContext<R>
            ) -> ::tl::Result<Self> {
                $deserialize
            }
            
            fn deserialize_boxed<R: ::std::io::Read>(
                id: ::tl::parsing::ConstructorId,
                reader: &mut ::tl::parsing::ReadContext<R>
            ) -> ::tl::Result<Self> {
                $deserialize_box
            }
        }
    ).unwrap()
}

fn impl_item_struct(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
    variant_data: &ast::VariantData
) -> (P<ast::Expr>, P<ast::Expr>, P<ast::Expr>, P<ast::Expr>) {
    let tid_path = builder.expr().path().id(item.ident).id("TYPE_ID").build();
    let type_id = quote_expr!(cx,
        Some($tid_path)
    );
    
    let serialize = match *variant_data {
        ast::VariantData::Unit(_) => {
            quote_expr!(cx,
                Ok(())
            )
        }
        ast::VariantData::Tuple(ref fields, _) => {
            let field_names: Vec<ast::Ident> = (0..fields.len())
                .map(|i| builder.id(format!("__field{}", i)))
                .collect();
            
            let pat = builder.pat().enum_()
                .id(item.ident).build()
                .with_pats(
                    field_names.iter().map(|field| builder.pat().ref_id(field))
                )
                .build();
            
            let ser_fields = builder.block().with_stmts(field_names.iter().map(|field| {
                quote_stmt!(cx,
                    try!(writer.write_generic($field));
                ).unwrap()
            })).build();
            
            quote_expr!(cx, {
                let $pat = *self;
                $ser_fields
                Ok(())
            })
        }
        ast::VariantData::Struct(ref fields, _) => {
            let field_names: Vec<ast::Ident> = (0..fields.len())
                .map(|i| builder.id(format!("__field{}", i)))
                .collect();
            
            let pat = builder.pat().struct_()
                .id(item.ident).build()
                .with_pats(
                    fields.iter().zip(field_names.iter()).map(|(field, field_name)| {
                        let name = match field.node.kind {
                            ast::NamedField(name, _) => name,
                            ast::UnnamedField(_) => {
                                cx.bug("struct variant has unnamed fields")
                            }
                        };
                        
                        (name, builder.pat().ref_id(field_name))
                    })
                )
                .build();
            
            let ser_fields = builder.block().with_stmts(field_names.iter().map(|field| {
                quote_stmt!(cx,
                    try!(writer.write_generic($field));
                ).unwrap()
            })).build();
            
            quote_expr!(cx, {
                let $pat = *self;
                $ser_fields
                Ok(())
            })
        }
    };
    
    let deserialize = match *variant_data {
        ast::VariantData::Unit(_) => {
            let var = builder.path().id(item.ident).build();
            quote_expr!(cx,
                Ok($var)
            )
        }
        ast::VariantData::Tuple(ref fields, _) => {
            let pat = builder.expr().call()
                .path().id(item.ident).build()
                .with_args((0..fields.len()).map(|_| {
                    quote_expr!(cx, try!(reader.read_generic()))
                }))
                .build();
            
            quote_expr!(cx,
                Ok($pat)
            )
        }
        ast::VariantData::Struct(ref fields, _) => {
            let pat = builder.expr().struct_()
                .id(item.ident).build()
                .with_id_exprs(fields.iter().map(|field| {
                    let name = match field.node.kind {
                        ast::NamedField(name, _) => name,
                        ast::UnnamedField(_) => {
                            cx.bug("struct variant has unnamed fields")
                        }
                    };
                    
                    (name, quote_expr!(cx, try!(reader.read_generic())))
                }))
                .build();
            
            quote_expr!(cx,
                Ok($pat)
            )
        }
    };
    
    let deserialize_box = quote_expr!(cx, {
        if id == $tid_path {
            Self::deserialize(reader)
        } else {
            Err(::tl::error::Error::InvalidType)
        }
    });
    
    (type_id, serialize, deserialize, deserialize_box)
}

fn impl_item_enum(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    enum_def: &ast::EnumDef
) -> (P<ast::Expr>, P<ast::Expr>, P<ast::Expr>, P<ast::Expr>) {
    let type_id = impl_enum_type_id(
        cx,
        builder,
        type_ident,
        enum_def
    );
    
    let serialize = impl_enum_serialize(
        cx,
        builder,
        type_ident,
        enum_def
    );
    
    let deserialize = quote_expr!(cx, {
        let _ = reader;
        Err(::tl::error::Error::BoxedAsBare)
    });
    
    let deserialize_box = impl_enum_deserialize(
        cx,
        builder,
        type_ident,
        enum_def
    );
    
    (type_id, serialize, deserialize, deserialize_box)
}

fn impl_enum_type_id(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    enum_def: &ast::EnumDef
) -> P<ast::Expr> {
    let arms: Vec<ast::Arm> = enum_def.variants.iter()
        .map(|variant| {
            impl_enum_type_id_arm(
                cx,
                builder,
                type_ident,
                variant,
            )
        })
        .collect();
        
    quote_expr!(cx,
        match *self {
            $arms
        }
    )
}

fn impl_enum_type_id_arm(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    variant: &ast::Variant,
) -> ast::Arm {
    let variant_ident = variant.node.name;
    
    let var_id = variant.node.attrs.iter().filter_map(|attr| {
        if let ast::MetaList(ref n, ref list) = attr.node.value.node {
            if &**n == "tl_id" {
                if let ast::MetaWord(ref n) = list[0].node {
                    let id = u32::from_str_radix(&(**n)[1..], 16).unwrap();
                    return Some(id);
                }
            }
        }
        None
    }).next().unwrap();
    
    let pat = match variant.node.data {
        ast::VariantData::Unit(_) => {
            builder.pat().enum_()
                .id(type_ident).id(variant_ident)
                .build().build()
        }
        ast::VariantData::Tuple(ref fields, _) => {
            let unused_field = builder.id("_");
            builder.pat().enum_()
                .id(type_ident).id(variant_ident).build()
                .with_pats(
                    fields.iter().map(|_| builder.pat().id(&unused_field))
                )
                .build()
        }
        ast::VariantData::Struct(ref fields, _) => {
            let unused_field = builder.id("_");
            builder.pat().struct_()
                .id(type_ident).id(variant_ident).build()
                .with_pats(
                    fields.iter().map(|field| {
                        let name = match field.node.kind {
                            ast::NamedField(name, _) => name,
                            ast::UnnamedField(_) => {
                                cx.bug("struct variant has unnamed fields")
                            }
                        };
                        
                        (name, builder.pat().id(&unused_field))
                    })
                )
                .build()
        }
    };
            
    quote_arm!(cx,
        $pat => {
            Some(::tl::parsing::ConstructorId($var_id))
        }
    )
}

fn impl_enum_serialize(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    enum_def: &ast::EnumDef
) -> P<ast::Expr> {
    let arms: Vec<ast::Arm> = enum_def.variants.iter()
        .map(|variant| {
            impl_enum_serialize_arm(
                cx,
                builder,
                type_ident,
                variant,
            )
        })
        .collect();
        
    quote_expr!(cx,
        match *self {
            $arms
        }
    )
}

fn impl_enum_serialize_arm(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    variant: &ast::Variant,
) -> ast::Arm {
    let variant_ident = variant.node.name;
    
    match variant.node.data {
        ast::VariantData::Unit(_) => {
            let pat = builder.pat().enum_()
                .id(type_ident).id(variant_ident)
                .build().build();
            
            quote_arm!(cx,
                $pat => {
                    Ok(())
                }
            )
        }
        ast::VariantData::Tuple(ref fields, _) => {
            let field_names: Vec<ast::Ident> = (0..fields.len())
                .map(|i| builder.id(format!("__field{}", i)))
                .collect();
            
            let pat = builder.pat().enum_()
                .id(type_ident).id(variant_ident).build()
                .with_pats(
                    field_names.iter().map(|field| builder.pat().ref_id(field))
                )
                .build();
            
            let ser_fields = builder.block().with_stmts(field_names.iter().map(|field| {
                quote_stmt!(cx,
                    try!(writer.write_generic($field));
                ).unwrap()
            })).build();
            
            quote_arm!(cx,
                $pat => {
                    $ser_fields
                    Ok(())
                }
            )
        }
        ast::VariantData::Struct(ref fields, _) => {
            let field_names: Vec<ast::Ident> = (0..fields.len())
                .map(|i| builder.id(format!("__field{}", i)))
                .collect();
            
            let pat = builder.pat().struct_()
                .id(type_ident).id(variant_ident).build()
                .with_pats(
                    fields.iter().zip(field_names.iter()).map(|(field, field_name)| {
                        let name = match field.node.kind {
                            ast::NamedField(name, _) => name,
                            ast::UnnamedField(_) => {
                                cx.bug("struct variant has unnamed fields")
                            }
                        };
                        
                        (name, builder.pat().ref_id(field_name))
                    })
                )
                .build();
            
            let ser_fields = builder.block().with_stmts(field_names.iter().map(|field| {
                quote_stmt!(cx,
                    try!(writer.write_generic($field));
                ).unwrap()
            })).build();
            
            quote_arm!(cx,
                $pat => {
                    $ser_fields
                    Ok(())
                }
            )
        }
    }
}

fn impl_enum_deserialize(
    cx: &mut ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    enum_def: &ast::EnumDef
) -> P<ast::Expr> {
    let arms: Vec<ast::Arm> = enum_def.variants.iter()
        .map(|variant| {
            impl_enum_deserialize_arm(
                cx,
                builder,
                type_ident,
                variant,
            )
        })
        .collect();
        
    quote_expr!(cx,
        match id.0 {
            $arms
            _ => Err(::tl::error::Error::InvalidType)
            // TODO
        }
    )
}

fn impl_enum_deserialize_arm(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    type_ident: ast::Ident,
    variant: &ast::Variant,
) -> ast::Arm {
    let variant_ident = variant.node.name;
    
    let var_id = variant.node.attrs.iter().filter_map(|attr| {
        if let ast::MetaList(ref n, ref list) = attr.node.value.node {
            if &**n == "tl_id" {
                if let ast::MetaWord(ref n) = list[0].node {
                    let id = u32::from_str_radix(&(**n)[1..], 16).unwrap();
                    return Some(id);
                }
            }
        }
        None
    }).next().unwrap();
    
    match variant.node.data {
        ast::VariantData::Unit(_) => {
            let var = builder.path().id(type_ident).id(variant_ident).build();
            quote_arm!(cx,
                $var_id => {
                    Ok($var)
                }
            )
        }
        ast::VariantData::Tuple(ref fields, _) => {
            let pat = builder.expr().call()
                .path().id(type_ident).id(variant_ident).build()
                .with_args((0..fields.len()).map(|_| {
                    quote_expr!(cx, try!(reader.read_generic()))
                }))
                .build();
            
            quote_arm!(cx,
                $var_id => {
                    Ok($pat)
                }
            )
        }
        ast::VariantData::Struct(ref fields, _) => {
            let pat = builder.expr().struct_()
                .id(type_ident).id(variant_ident).build()
                .with_id_exprs(fields.iter().map(|field| {
                    let name = match field.node.kind {
                        ast::NamedField(name, _) => name,
                        ast::UnnamedField(_) => {
                            cx.bug("struct variant has unnamed fields")
                        }
                    };
                    
                    (name, quote_expr!(cx, try!(reader.read_generic())))
                }))
                .build();
            
            quote_arm!(cx,
                $var_id => {
                    Ok($pat)
                }
            )
        }
    }
}

pub fn register(registry: &mut Registry) {
    registry.add_attr("feature(custom_derive)");
    registry.add_attr("feature(custom_attribute)");
    registry.add_decorator("derive_TLType", ComplexExpander);
    registry.add_decorator("tl_id", TLIdExpander);
    registry.add_post_expansion_pass(strip_attributes);
}

fn strip_attributes(krate: ast::Crate) -> ast::Crate {
    use syntax::{ast, fold};
    struct StripAttributeFolder;

    impl fold::Folder for StripAttributeFolder {
        fn fold_attribute(&mut self, attr: ast::Attribute) -> Option<ast::Attribute> {
            match attr.node.value.node {
                ast::MetaList(ref n, _) if n == &"tl_id" => { return None; }
                _ => {}
            }

            Some(attr)
        }

        fn fold_mac(&mut self, mac: ast::Mac) -> ast::Mac {
            fold::noop_fold_mac(mac, self)
        }
    }

    fold::Folder::fold_crate(&mut StripAttributeFolder, krate)
}
