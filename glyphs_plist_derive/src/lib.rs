extern crate proc_macro;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, LitStr};

#[derive(Debug)]
enum PlistAttribute {
    Standard(PlistAttributeInner),
    Rest,
    None,
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
                            inner.default = Some(expr)
                        }
                        Err(_) => {
                            // Presume the error was there not being an = and expr, use default
                            inner.use_default_trait();
                        }
                    };
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
    default: Option<TokenStream>,
}

impl PlistAttributeInner {
    fn unused(&self) -> bool {
        self.serialised_name.is_none() && self.default.is_none()
    }

    fn use_default_trait(&mut self) {
        self.default = Some(quote! { Default::default() });
    }
}

#[proc_macro_derive(FromPlist, attributes(plist, rest, rename))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let DeserialisedFields {
        fields,
        consumes_rest,
    } = add_deser(&input.data);

    let expanded = if consumes_rest {
        quote! {
            impl crate::from_plist::FromPlist for #name {
                fn from_plist(plist: crate::plist::Plist) -> Self {
                    let mut hashmap = plist.into_hashmap();
                    #name {
                        #fields
                    }
                }
            }
        }
    } else {
        quote! {
            impl crate::from_plist::FromPlist for #name {
                fn from_plist(plist: crate::plist::Plist) -> Self {
                    let mut hashmap = plist.into_hashmap();
                    let result = #name {
                        #fields
                    };
                    assert!(hashmap.is_empty(), "unrecognised fields in {}: {:?}", stringify!(#name), hashmap.keys());
                    result
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(ToPlist, attributes(rest, rename))]
pub fn derive_to(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let ser_rest = add_ser_rest(&input.data);
    let ser = add_ser(&input.data);

    let expanded = quote! {
        impl crate::to_plist::ToPlist for #name {
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
            match options {
                PlistAttribute::Standard(PlistAttributeInner {
                    serialised_name,
                    default,
                }) => {
                    let plist_name = serialised_name.unwrap_or_else(camel_case_field_name);
                    let tokens = match default {
                        Some(default) => quote_spanned! {field.span()=>
                            #field_name: hashmap.remove(#plist_name)
                                .map(crate::from_plist::FromPlist::from_plist)
                                .unwrap_or_else(|| #default),
                        },
                        None => {
                            quote_spanned! {field.span()=>
                                #field_name: crate::from_plist::FromPlistOpt::from_plist(
                                    hashmap.remove(#plist_name)
                                ),
                            }
                        }
                    };
                    Some(tokens)
                }
                PlistAttribute::None => {
                    let plist_name = camel_case_field_name();
                    Some(quote_spanned! {field.span()=>
                        #field_name: crate::from_plist::FromPlistOpt::from_plist(
                            hashmap.remove(#plist_name)
                        ),
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
        .filter_map(|(field, options)| {
            let field_name = field.ident.as_ref().unwrap();
            let plist_name = match options {
                PlistAttribute::Standard(PlistAttributeInner {
                    serialised_name: Some(plist_name),
                    ..
                }) => plist_name,
                PlistAttribute::Standard(PlistAttributeInner {
                    serialised_name: None,
                    ..
                })
                | PlistAttribute::None => field_name.unraw().to_string().to_lower_camel_case(),
                PlistAttribute::Rest => return None,
            };
            Some(quote_spanned! {field.span()=>
                if let Some(plist) = crate::to_plist::ToPlistOpt::to_plist(self.#field_name) {
                    hashmap.insert(String::from(#plist_name), plist);
                }
            })
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
