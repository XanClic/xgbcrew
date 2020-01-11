extern crate proc_macro;
extern crate quote;
extern crate syn;

use quote::quote;

use crate::proc_macro::TokenStream;


#[proc_macro_derive(SaveState, attributes(savestate))]
pub fn save_state_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    match ast.data {
        syn::Data::Struct(s) => {
            match s.fields {
                syn::Fields::Named(sf) =>
                    save_state_derive_struct(ast.ident, sf),

                _ => panic!("Not implemented yet"),
            }
        },

        _ => panic!("Not implemented yet"),
    }
}

struct Attr {
    name: syn::Ident,
    post_import: Vec<syn::Expr>,
    neg_conditions: Vec<syn::Expr>,
}

fn save_state_derive_struct(name: syn::Ident, sf: syn::FieldsNamed)
    -> TokenStream
{
    let mut v = Vec::<Attr>::new();

    for field in &sf.named {
        let mut post_import = Vec::new();
        let mut neg_conditions = Vec::new();
        let field_name = field.ident.as_ref().unwrap().to_string();
        let mut skip = false;

        for a in &field.attrs {
            let attr_name = a.path.get_ident().as_ref().unwrap().to_string();

            if attr_name != "savestate" {
                continue;
            }

            match a.parse_meta().unwrap() {
                syn::Meta::List(l) => {
                    for opt in l.nested {
                        match opt {
                            syn::NestedMeta::Meta(m) => {
                                match m {
                                    syn::Meta::Path(p) => {
                                        let opt_name = p.get_ident().as_ref().unwrap().to_string();

                                        if opt_name == "skip" {
                                            skip = true;
                                        } else {
                                            panic!("Unknown option {} for field {}",
                                                   opt_name, field_name);
                                        }
                                    },

                                    syn::Meta::List(l) => {
                                        let opt_name = l.path.get_ident().as_ref().unwrap().to_string();

                                        for opt in l.nested {
                                            match opt {
                                                syn::NestedMeta::Lit(syn::Lit::Str(ls)) => {
                                                    if opt_name == "post_import" {
                                                        let s = syn::parse_str::<syn::Expr>(&ls.value()).unwrap();
                                                        post_import.push(s);
                                                    } else if opt_name == "skip_if" {
                                                        let s = syn::parse_str::<syn::Expr>(&ls.value()).unwrap();
                                                        neg_conditions.push(s);
                                                    } else {
                                                        panic!("Unknown option {} for field {}",
                                                               opt_name, field_name);
                                                    }
                                                },

                                                _ => panic!("Invalid syntax"),
                                            }
                                        }
                                    },

                                    _ => panic!("Invalid syntax"),
                                }
                            },

                            _ => panic!("Invalid syntax"),
                        };
                    }
                }

                _ => panic!("Invalid syntax"),
            }
        }

        if skip {
            continue;
        }

        v.push(Attr {
            name: field.ident.as_ref().unwrap().clone(),
            post_import: post_import,
            neg_conditions: neg_conditions,
        });
    }

    let export_list = v.iter().map(|attr| {
        let name = &attr.name;
        let ncond = &attr.neg_conditions;

        if ncond.is_empty() {
            quote! {
                savestate::SaveState::export(&self.#name, stream, version);
            }
        } else {
            quote! {
                if #(!(#ncond))&&* {
                    savestate::SaveState::export(&self.#name, stream, version);
                }
            }
        }
    }).collect::<Vec<quote::__rt::TokenStream>>();

    let import_list = v.iter().map(|attr| {
        let name = &attr.name;
        let ncond = &attr.neg_conditions;
        let post = &attr.post_import;

        if ncond.is_empty() {
            quote! {
                savestate::SaveState::import(&mut self.#name, stream, version);
                #(#post;)*
            }
        } else {
            quote! {
                if #(!(#ncond))&&* {
                    savestate::SaveState::import(&mut self.#name, stream,
                                                 version);
                    #(#post;)*
                }
            }
        }
    }).collect::<Vec<quote::__rt::TokenStream>>();

    let result = quote! {
        impl savestate::SaveState for #name {
            fn export<T: std::io::Write>(&self, stream: &mut T, version: u64) {
                #(#export_list)*
            }

            fn import<T: std::io::Read>(&mut self, stream: &mut T,
                                        version: u64)
            {
                #(#import_list)*
            }
        }
    };
    result.into()
}
