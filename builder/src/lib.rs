use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Type};

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

    let builder_fn = builder_fn(&ident, &builder_ident, &struct_fields);
    let builder_struct = builder_struct(&ident, &builder_ident, &struct_fields, &struct_types);

    let combined = quote! {
        #builder_fn
        #builder_struct
    };

    proc_macro::TokenStream::from(combined)
}

fn builder_fn(ident: &Ident, builder_ident: &Ident, struct_fields: &[Ident]) -> TokenStream {
    quote! {
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#struct_fields: None),*
                }
            }
        }
    }
}

fn builder_struct(
    ident: &Ident,
    builder_ident: &Ident,
    struct_fields: &[Ident],
    struct_types: &[Type],
) -> TokenStream {
    // Verify that each non-optional field was set
    let (optional_fields, optional_types) = optional_fields(struct_fields, struct_types);
    let (required_fields, required_types): (Vec<Ident>, Vec<Type>) = struct_fields
        .iter()
        .cloned()
        .zip(struct_types.iter().cloned())
        .filter(|(id, _ty)| !optional_fields.contains(id))
        .unzip();
    let field_checkers: Vec<TokenStream> = required_fields
        .iter()
        .map(|id| {
            let err_str = format!("Field {id} was never set");
            quote! { if self.#id.is_none() {
                    return Err(String::from(#err_str).into())
                }
            }
        })
        .collect();

    let required_field_setters = quote! {
        #(
            fn #required_fields(&mut self, #required_fields: #required_types) -> &mut Self {
                self.#required_fields = Some(#required_fields);
                self
            }
        )*
    };

    let optional_field_setters = quote! {
        #(
            fn #optional_fields(&mut self, #optional_fields: #optional_types) -> &mut Self {
                self.#optional_fields = Some(Some(#optional_fields));
                self
            }
        )*
    };

    quote! {
        pub struct #builder_ident {
            #(#struct_fields: Option<#struct_types>),*
        }

        impl #builder_ident {
            // Build method
            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                // Check all fields are set
                #(#field_checkers)*

                Ok(
                    #ident {
                        #(#required_fields: self.#required_fields.clone().unwrap()),*,
                        #(#optional_fields: self.#optional_fields.clone().unwrap_or(None)),*
                    }
                )
            }

            // Setter methods
            #required_field_setters
            #optional_field_setters
        }
    }
}

fn optional_fields(struct_fields: &[Ident], struct_types: &[Type]) -> (Vec<Ident>, Vec<Type>) {
    let mut optionals = vec![];
    let mut types = vec![];

    for (id, ty) in struct_fields.iter().zip(struct_types.iter()) {
        if let Type::Path(type_path) = ty {
            if type_path.qself.is_some() {
                continue;
            }
            let segments = &type_path.path.segments;
            if let Some(ps) = segments.first() {
                if ps.ident != "Option" {
                    continue;
                }
                if let syn::PathArguments::AngleBracketed(generic_arg) = &ps.arguments {
                    if let Some(syn::GenericArgument::Type(ty)) = generic_arg.args.first() {
                        optionals.push(id.clone());
                        types.push(ty.clone());
                    }
                }
            }
        }
    }

    (optionals, types)
}
