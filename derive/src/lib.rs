use proc_macro::TokenStream;
use syn::{punctuated::Punctuated, token::Comma, DeriveInput, Field, Generics, Ident};

#[proc_macro_derive(HasPartial)]
pub fn has_partial(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = syn::parse_macro_input!(input as DeriveInput);
    // TODO: support renaming partial
    // TODO: support inheriting `pub(crate)
    // TODO: warn on private structs
    // TODO: panic on generics

    let partial_ident = quote::format_ident!("Partial{}", ident);

    let strct = match data {
        syn::Data::Struct(thing) => thing,
        syn::Data::Enum(_) => panic!("Enums are not supported"),
        syn::Data::Union(_) => panic!("Data unions are not supported"),
    };

    let fields = match strct.fields {
        syn::Fields::Named(namede) => namede.named,
        _ => unreachable!(),
    };
    let (optional_fields, required_fields): (Punctuated<Field, Comma>, Punctuated<Field, Comma>) =
        fields.into_iter().partition(|field| is_option(&field.ty));

    let required_fields: Punctuated<Field, Comma> = required_fields
        .into_iter()
        .map(|field| {
            let ty = field.ty;
            let ty: syn::Type = syn::parse_quote! { Option<#ty>};
            Field { ty, ..field }
        })
        .collect();

    let impl_has_partial = quote::quote! {
        impl #generics ::partial_config::HasPartial for #ident #generics {
            type Partial = #partial_ident #generics;
        }
    };

    let impl_partial = impl_partial(
        &generics,
        &ident,
        &partial_ident,
        &required_fields,
        &optional_fields,
    )
    .unwrap();

    let all_fields: Punctuated<Field, Comma> = optional_fields
        .iter()
        .cloned()
        .chain(required_fields.iter().cloned())
        .collect();

    let derives: syn::Attribute = {
        #[cfg(feature = "serde")]
        syn::parse_quote! {
            #[derive(Debug, Default, ::serde::Deserialize)]
        }
        #[cfg(not(feature = "serde"))]
        syn::parse_quote! {
            #[derive(Debug, Default)]
        }
    };

    let output = quote::quote! {
        #derives
        pub struct #partial_ident #generics {
            #all_fields
        }

        #[automatically_derived]
        #impl_partial

        #[automatically_derived]
        #impl_has_partial
    };

    TokenStream::from(output)
}

fn impl_partial(
    generics: &Generics,
    ident: &Ident,
    partial_ident: &Ident,
    required_fields: &Punctuated<Field, Comma>,
    optional_fields: &Punctuated<Field, Comma>,
) -> Result<proc_macro2::TokenStream, &'static str> {
    let error: syn::Expr = syn::parse_quote! {
        Err(::partial_config::Error::MissingFields {
            required_fields: missing_fields
        })
    };

    let opt_fields: Punctuated<Ident, Comma> = optional_fields
        .iter()
        .cloned()
        .filter_map(|field| field.ident)
        .collect();

    let req_fields: Punctuated<Ident, Comma> = required_fields
        .iter()
        .cloned()
        .filter_map(|field| field.ident)
        .collect();

    let assembling_config: syn::Stmt = assembling_config(req_fields.len(), opt_fields.len());

    let req_field_expr: Punctuated<syn::Stmt, syn::token::Semi> = req_fields
        .iter()
        .cloned()
        .map(|ident| -> syn::Stmt {
            syn::parse_quote! {
                let #ident = match self.#ident {
                    Some(value) => value,
                    None => {
                        missing_fields.push(::partial_config::MissingField(stringify!(#ident)));
                        Default::default()
                    }
                };
            }
        })
        .collect();

    let opt_field_expr: Punctuated<syn::Stmt, syn::token::Semi> = optional_fields
        .iter()
        .cloned()
        .filter_map(|field: Field| {
            field.ident.map(|ident| -> syn::Stmt {
                // TODO: add explicit fallback
                syn::parse_quote! {
                    let #ident = self.#ident;
                }
            })
        })
        .collect();

    let all_fields: Punctuated<Ident, Comma> = opt_fields
        .into_iter()
        .chain(req_fields.into_iter())
        .collect();

    let override_expr: Punctuated<syn::Stmt, syn::token::Semi> = all_fields
        .iter()
        .cloned()
        .map(|ident: Ident| -> syn::Stmt {
            syn::parse_quote! {
                let #ident = other.#ident.or(self.#ident);
            }
        })
        .collect();

    Ok(quote::quote! {
        impl #generics ::partial_config::Partial for #partial_ident #generics {
            type Target = #ident #generics;

            type Error = ::partial_config::Error;

            fn build(self) -> Result<Self::Target, Self::Error> {
                let mut missing_fields = ::std::vec::Vec::new();
                #assembling_config;

                #req_field_expr
                #opt_field_expr

                if !missing_fields.is_empty() {
                    #error
                } else {
                    Ok(
                        Self::Target {
                            #all_fields
                        }
                    )
                }
            }

            fn override_with(self, other: Self) -> Self {
                #override_expr
                Self {
                    #all_fields
                }

            }
        }
    })
}

fn is_option(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident == "Option")
            .unwrap_or(false),
        _ => false,
    }
}

#[cfg(all(feature = "tracing", feature = "log"))]
compile_error!("The features \"tracing\" and \"log\" are mutually exclusive. Please either use pure tracing, or enable the \"log\" feature in \"tracing\" and use the \"log\" feature of this crate. ");

fn assembling_config(required_fields_count: usize, optional_fields_count: usize) -> syn::Stmt {
    #[cfg(feature = "tracing")]
    syn::parse_quote! {
        {
            ::tracing::info!(?self, "Building configuration {required_fields_count} ({optional_fields_count})", required_fields_count = #required_fields_count, optional_fields_count=#optional_fields_count);
        }
    }
    #[cfg(feature = "log")]
    syn::parse_quote! {
        ::log::info!("Building configuration. {required_fields_count} ({optional_fields_count}) fields", required_fields_count = #required_fields_count, optional_fields_count=#optional_fields_count);
    }
    #[cfg(not(any(feature = "tracing", feature = "log")))]
    syn::parse_quote! {
        println!("Building configuration. {required_fields_count} ({optional_fields_count}) fields", required_fields_count = #required_fields_count, optional_fields_count=#optional_fields_count);
    }
}
