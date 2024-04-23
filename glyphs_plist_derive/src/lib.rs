extern crate proc_macro;

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, LitStr};

#[proc_macro_derive(FromPlist, attributes(rest, rename, default))]
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
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields
                    .named
                    .iter()
                    .filter(|field| !is_rest(&field.attrs))
                    .map(|f| {
                        let name = &f.ident;
                        let name_str = name.as_ref().unwrap().to_string();
                        let plist_name =
                            get_name(&f.attrs).unwrap_or_else(|| name_str.to_lower_camel_case());
                        let default = get_default(&f.attrs);
                        match default {
                            Some(default) => quote_spanned! {f.span() =>
                                #[allow(unused_parens, clippy::double_parens)]
                                #name: hashmap
                                    .remove(#plist_name)
                                    .map(Some)
                                    .map(crate::from_plist::FromPlistOpt::from_plist)
                                    .unwrap_or_else(|| #default),
                            },
                            None => quote_spanned! {f.span() =>
                                #name: crate::from_plist::FromPlistOpt::from_plist(
                                    hashmap.remove(#plist_name)
                                ),
                            },
                        }
                    });

                let recurse_rest = fields.named.iter().find_map(|f| {
                    if is_rest(&f.attrs) {
                        let name = &f.ident;
                        Some(quote_spanned! {f.span() =>
                            #name: hashmap,
                        })
                    } else {
                        None
                    }
                });

                match recurse_rest {
                    Some(recurse_rest) => DeserialisedFields {
                        fields: quote! {
                            #( #recurse )*
                            #recurse_rest
                        },
                        consumes_rest: true,
                    },
                    None => DeserialisedFields {
                        fields: quote! { #( #recurse )* },
                        consumes_rest: false,
                    },
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn add_ser(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().filter_map(|f| {
                    if !is_rest(&f.attrs) {
                        let name = &f.ident;
                        let name_str = name.as_ref().unwrap().to_string();
                        let plist_name =
                            get_name(&f.attrs).unwrap_or_else(|| name_str.to_lower_camel_case());
                        Some(quote_spanned! {f.span() =>
                            if let Some(plist) = crate::to_plist::ToPlistOpt::to_plist(self.#name) {
                                hashmap.insert(#plist_name.to_string(), plist);
                            }
                        })
                    } else {
                        None
                    }
                });
                quote! {
                    #( #recurse )*
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn add_ser_rest(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                for f in fields.named.iter() {
                    if is_rest(&f.attrs) {
                        let name = &f.ident;
                        return quote_spanned! { f.span() =>
                            let mut hashmap = self.#name;
                        };
                    }
                }
                quote! { let mut hashmap = HashMap::new(); }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn is_rest(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .get_ident()
            .map(|ident| ident == "rest")
            .unwrap_or(false)
    })
}

fn get_name(attrs: &[Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident("rename"))
        .map(|attr| {
            attr.parse_args::<LitStr>()
                .expect("Could not parse 'rename' attribute as string literal")
                .value()
        })
}

fn get_default(attrs: &[Attribute]) -> Option<TokenStream> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident("default"))
        .map(|attr| attr.to_token_stream())
}
