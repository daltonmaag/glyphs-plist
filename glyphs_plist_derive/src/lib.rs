extern crate proc_macro;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::mem;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, LitStr, Path, Type, TypePath};

#[derive(Debug)]
enum PlistAttribute {
    Standard(PlistAttributeInner),
    Rest,
    None,
}

impl PlistAttribute {
    fn always_serialise(&self) -> bool {
        if let PlistAttribute::Standard(inner) = self {
            inner.always_serialise
        } else {
            false
        }
    }

    fn take_default_to_tokens(&mut self, type_path: &Path) -> Option<TokenStream> {
        if let PlistAttribute::Standard(inner) = self {
            inner.default.take_tokens(type_path)
        } else {
            None
        }
    }

    fn take_serialised_name(&mut self) -> Option<String> {
        if let PlistAttribute::Standard(inner) = self {
            inner.serialised_name.take()
        } else {
            None
        }
    }
}

impl From<&[Attribute]> for PlistAttribute {
    fn from(attrs: &[Attribute]) -> Self {
        let Some(plist_attr) = attrs.iter().find(|attr| attr.path().is_ident("plist")) else {
            return PlistAttribute::None;
        };
        let mut rest = false;
        let mut inner = PlistAttributeInner::default();
        plist_attr
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("rest") {
                    rest = true;
                    return Ok(());
                }
                if meta.path.is_ident("rename") {
                    let name = meta.value()?.parse::<LitStr>()?;
                    inner.serialised_name = Some(name.value());
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    match meta.value() {
                        // Expression provided, use it
                        Ok(stream) => {
                            let expr = stream.parse::<TokenStream>()?;
                            inner.default = PlistAttributeDefault::Expr(expr)
                        }
                        Err(_) => {
                            // Presume the error was there not being an = and expr, use default
                            // trait
                            inner.default = PlistAttributeDefault::DefaultTrait;
                        }
                    };
                    return Ok(());
                }
                if meta.path.is_ident("always_serialize") || meta.path.is_ident("always_serialise")
                {
                    inner.always_serialise = true;
                    return Ok(());
                }
                Err(meta.error("missing/unrecognised plist attribute(s)"))
            })
            .unwrap_or_else(|err| {
                panic!("bad plist attribute: {err}");
            });
        if rest {
            debug_assert!(
                inner.unused(),
                "plist(rest) should not be used with other attributes",
            );
            PlistAttribute::Rest
        } else if !inner.unused() {
            PlistAttribute::Standard(inner)
        } else {
            // Attribute given, but with no options (thanks)
            PlistAttribute::None
        }
    }
}

#[derive(Debug, Default)]
struct PlistAttributeInner {
    serialised_name: Option<String>,
    default: PlistAttributeDefault,
    always_serialise: bool,
}

impl PlistAttributeInner {
    fn unused(&self) -> bool {
        matches!(
            self,
            PlistAttributeInner {
                serialised_name: None,
                default: PlistAttributeDefault::None,
                always_serialise: false
            }
        )
    }
}

#[derive(Debug, Default)]
enum PlistAttributeDefault {
    Expr(TokenStream),
    DefaultTrait,
    #[default]
    None,
}

impl PlistAttributeDefault {
    fn take_tokens(&mut self, type_path: &Path) -> Option<TokenStream> {
        let mut old_self = PlistAttributeDefault::default();
        mem::swap(self, &mut old_self);
        match old_self {
            PlistAttributeDefault::Expr(expr) => Some(expr),
            PlistAttributeDefault::DefaultTrait => Some(quote! { <#type_path>::default() }),
            PlistAttributeDefault::None => None,
        }
    }
}

#[proc_macro_derive(FromPlist, attributes(plist))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let DeserialisedFields {
        fields,
        consumes_rest,
    } = add_deser(&input.data);

    let expanded = if consumes_rest {
        quote! {
            impl TryFrom<crate::plist::Plist> for #name {
                type Error = crate::GlyphsFromPlistError;

                #[allow(clippy::unnecessary_fallible_conversions)]
                fn try_from(plist: crate::plist::Plist) -> Result<Self, Self::Error> {
                    let mut hashmap = plist.into_hashmap();
                    Ok(#name {
                        #fields
                    })
                }
            }
        }
    } else {
        quote! {
            impl TryFrom<crate::plist::Plist> for #name {
                type Error = crate::GlyphsFromPlistError;

                #[allow(clippy::unnecessary_fallible_conversions)]
                fn try_from(plist: crate::plist::Plist) -> Result<Self, Self::Error> {
                    let mut hashmap = plist.into_hashmap();
                    let result = #name {
                        #fields
                    };
                    assert!(hashmap.is_empty(), "unrecognised fields in {}: {:?}", stringify!(#name), hashmap.keys());
                    Ok(result)
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(ToPlist, attributes(plist))]
pub fn derive_to(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let ser_rest = add_ser_rest(&input.data);
    let ser = add_ser(&input.data);

    let expanded = quote! {
        impl crate::to_plist::ToPlist for #name {
            #[allow(clippy::bool_comparison)]
            fn to_plist(self) -> crate::plist::Plist {
                #ser_rest
                #ser
                hashmap.into()
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

struct DeserialisedFields {
    fields: TokenStream,
    consumes_rest: bool,
}

fn add_deser(data: &Data) -> DeserialisedFields {
    let Data::Struct(data) = data else {
        unimplemented!("only structs");
    };
    let Fields::Named(fields) = &data.fields else {
        unimplemented!("only structs with named fields");
    };
    let recurse = fields
        .named
        .iter()
        .map(|field| (field, PlistAttribute::from(field.attrs.as_slice())))
        .filter_map(|(field, options)| {
            let field_name = field.ident.as_ref().unwrap();
            let camel_case_field_name = || {
                let unraw = field_name.unraw().to_string();
                unraw.to_lower_camel_case()
            };
            let field_name_str = field_name.to_string();
            let field_is_option = if let Type::Path(TypePath { path, .. }) = &field.ty {
                path.segments.first().unwrap().ident == "Option"
            } else {
                unreachable!("field type is always Type::Path")
            };
            match options {
                PlistAttribute::Standard(PlistAttributeInner {
                    serialised_name,
                    default,
                    ..
                }) => {
                    let plist_name = serialised_name.unwrap_or_else(camel_case_field_name);
                    let tokens = match default {
                        PlistAttributeDefault::Expr(default) => quote_spanned! {field.span()=>
                            #field_name: hashmap.remove(#plist_name)
                                .map_or_else(|| Ok(#default), TryFrom::try_from)?,
                        },
                        PlistAttributeDefault::DefaultTrait => quote_spanned! {field.span()=>
                            #field_name: hashmap.remove(#plist_name)
                                .map_or_else(|| Ok(Default::default()), TryFrom::try_from)?,
                        },
                        // TODO: de-dupe these two clauses with the pair below
                        PlistAttributeDefault::None if field_is_option => {
                            quote_spanned! {field.span()=>
                                #field_name: match hashmap.remove(#plist_name) {
                                    Some(plist) => Some(plist.try_into()?),
                                    None => None,
                                },
                            }
                        }
                        PlistAttributeDefault::None => {
                            quote_spanned! {field.span()=>
                                #field_name: match hashmap.remove(#plist_name) {
                                    Some(plist) => plist.try_into()?,
                                    None => return Err(
                                        crate::GlyphsFromPlistError::MissingField(#field_name_str)
                                    ),
                                },
                            }
                        }
                    };
                    Some(tokens)
                }
                PlistAttribute::None if field_is_option => {
                    let plist_name = camel_case_field_name();
                    Some(quote_spanned! {field.span()=>
                        #field_name: match hashmap.remove(#plist_name) {
                            Some(plist) => Some(plist.try_into()?),
                            None => None,
                        },
                    })
                }
                PlistAttribute::None => {
                    let plist_name = camel_case_field_name();
                    Some(quote_spanned! {field.span()=>
                        #field_name: match hashmap.remove(#plist_name) {
                            Some(plist) => plist.try_into()?,
                            None => return Err(
                                crate::GlyphsFromPlistError::MissingField(#field_name_str)
                            ),
                        },
                    })
                }
                PlistAttribute::Rest => None,
            }
        });
    // We have to put the #[plist(rest)] field in a separate variable to be able to interpolate it last,
    // because it takes ownership of the hashmap that we're extracting the other fields' values from
    let collect_rest = fields
        .named
        .iter()
        .find(|field| {
            matches!(
                PlistAttribute::from(field.attrs.as_slice()),
                PlistAttribute::Rest,
            )
        })
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            quote_spanned! {field.span()=>
                #field_name: hashmap,
            }
        });

    match collect_rest {
        Some(rest) => DeserialisedFields {
            fields: quote! {
                #( #recurse )*
                #rest
            },
            consumes_rest: true,
        },
        None => DeserialisedFields {
            fields: quote! { #( #recurse )* },
            consumes_rest: false,
        },
    }
}

fn add_ser(data: &Data) -> TokenStream {
    let Data::Struct(data) = data else {
        unimplemented!("only structs");
    };
    let Fields::Named(fields) = &data.fields else {
        unimplemented!("only structs with named fields");
    };
    let recurse = fields
        .named
        .iter()
        .map(|field| (field, PlistAttribute::from(field.attrs.as_slice())))
        .filter_map(|(field, mut options)| {
            if matches!(options, PlistAttribute::Rest) {
                return None;
            }
            let field_name = field.ident.as_ref().unwrap();
            let plist_name = options
                .take_serialised_name()
                .unwrap_or_else(|| field_name.unraw().to_string().to_lower_camel_case());

            // Simple base case, no conditions to handle
            if options.always_serialise() {
                Some(quote_spanned! {field.span()=>
                    if let Some(plist) = crate::to_plist::ToPlistOpt::to_plist(self.#field_name) {
                        hashmap.insert(String::from(#plist_name), plist);
                    }
                })
            } else {
                match &field.ty {
                    // Special case handling for floats
                    Type::Path(TypePath { path, .. })
                        if path.is_ident("f32") || path.is_ident("f64") =>
                    {
                        // We can compare floats with PartialEq, but we don't have Eq. The side
                        // effect of this (AFAICT) is basically that a field with a default value
                        // of NaN would always get serialised, even if unchanged (since NaN != NaN)
                        let default_value = options
                            .take_default_to_tokens(path)
                            .unwrap_or(quote_spanned! {field.span()=> <#path>::default() });
                        Some(quote_spanned! {field.span()=>
                            let #field_name = PartialEq::ne(&self.#field_name, &#default_value)
                                .then(|| crate::to_plist::ToPlistOpt::to_plist(self.#field_name))
                                .flatten();
                            if let Some(plist) = #field_name {
                                hashmap.insert(String::from(#plist_name), plist);
                            }
                        })
                    }
                    Type::Path(TypePath { path, .. }) => {
                        let default_value = options
                            .take_default_to_tokens(path)
                            .unwrap_or(quote_spanned! {field.span()=> <#path>::default() });
                        Some(quote_spanned! {field.span()=>
                            let #field_name = (self.#field_name != #default_value)
                                .then(|| crate::to_plist::ToPlistOpt::to_plist(self.#field_name))
                                .flatten();
                            if let Some(plist) = #field_name {
                                hashmap.insert(String::from(#plist_name), plist);
                            }
                        })
                    }
                    _ => unreachable!("struct field types should all be Type::Path"),
                }
            }
        });
    quote! {
        #( #recurse )*
    }
}

fn add_ser_rest(data: &Data) -> TokenStream {
    let Data::Struct(data) = data else {
        unimplemented!("only structs");
    };
    let Fields::Named(fields) = &data.fields else {
        unimplemented!("only structs with named fields");
    };
    fields
        .named
        .iter()
        .find(|field| {
            matches!(
                PlistAttribute::from(field.attrs.as_slice()),
                PlistAttribute::Rest,
            )
        })
        .map_or(quote! { let mut hashmap = HashMap::new(); }, |field| {
            let name = field.ident.as_ref().unwrap();
            quote_spanned! { field.span()=> let mut hashmap = self.#name; }
        })
}
