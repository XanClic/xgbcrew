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

fn save_state_derive_struct(name: syn::Ident, sf: syn::FieldsNamed)
    -> TokenStream
{
    let mut v = Vec::new();
    let mut post_import = Vec::new();

    for field in &sf.named {
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

        if !skip {
            v.push(field.ident.as_ref().unwrap());
        }
    }

    let result = quote! {
        impl savestate::SaveState for #name {
            fn export<T: std::io::Write>(&self, stream: &mut T) {
                #(savestate::SaveState::export(&self.#v, stream);)*
            }

            fn import<T: std::io::Read>(&mut self, stream: &mut T) {
                #(savestate::SaveState::import(&mut self.#v, stream);)*
                #(#post_import;)*
            }
        }
    };
    result.into()
}
