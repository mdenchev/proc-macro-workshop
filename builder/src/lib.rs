use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let builder_ident = format_ident!("{ident}Builder");
    let mut struct_fields = vec![];
    let mut struct_types = vec![];
    if let syn::Data::Struct(ds) = input.data {
        if let syn::Fields::Named(fields) = ds.fields {
            for field in fields.named.into_iter() {
                struct_fields.push(field.ident.unwrap());
                struct_types.push(field.ty);
            }
        } else {
            panic!("Builder macro only works on structs with named fields");
        }
    } else {
        panic!("Builder macro only works on structs");
    };

    let builder_fields: Vec<TokenStream> = struct_fields
        .iter()
        .zip(struct_types.iter())
        .map(|(id, ty)| quote! { #id: Option<#ty> })
        .collect();

    let field_setters: Vec<TokenStream> = struct_fields
        .iter()
        .zip(struct_types.iter())
        .map(|(id, ty)| {
            quote! { fn #id(&mut self, #id: #ty) -> &mut Self {
                self.#id = Some(#id);
                self
            }}
        })
        .collect();

    let field_checkers: Vec<TokenStream> = struct_fields
        .iter()
        .map(|id| {
            let err_str = format!("Field {id} was never set");
            quote! { if self.#id.is_none() {
                    return Err(String::from(#err_str).into())
                }
            }
        })
        .collect();

    let set_fields_in_build: Vec<TokenStream> = struct_fields
        .iter()
        .map(|id| {
            quote! { #id: self.#id.clone().unwrap() }
        })
        .collect();

    let builder_fn = quote! {
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#struct_fields: None),*
                }
            }
        }
    };

    let build_fn = quote! {
        pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
            // Check all fields are set
            #(#field_checkers)*

            Ok(
                #ident {
                    #(#set_fields_in_build),*
                }
            )
        }
    };

    let builder_st = quote! {
        pub struct #builder_ident {
            #(#builder_fields),*
        }

        impl #builder_ident {
            // Build
            #build_fn

            #(#field_setters)*
        }
    };

    let combined = quote! {
        #builder_fn
        #builder_st
    };

    proc_macro::TokenStream::from(combined)
}
