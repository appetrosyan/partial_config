use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
    punctuated::Punctuated,
    token::{Comma, Type},
    DeriveInput, Field, Generics, Ident, TypePath, Visibility,
};

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
        syn::Fields::Unnamed(_) => unreachable!(),
        syn::Fields::Unit => unreachable!(),
    };
    let (mut optional_fields, required_fields): (
        Punctuated<Field, Comma>,
        Punctuated<Field, Comma>,
    ) = fields.into_iter().partition(|field| is_option(&field.ty));

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
    optional_fields.extend(required_fields);

    let output = quote::quote! {
        #[derive(Debug, Default)]
        pub struct #partial_ident #generics {
            #optional_fields
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
    let assembling_config: syn::Stmt = syn::parse_quote! {todo!();};

    let error: syn::Expr = syn::parse_quote! {
        Err(::partial_config::Error::MissingFields {
            required_fields: missing_fields
        })
    };

    let optional_fields: Punctuated<Ident, Comma> = optional_fields
        .iter()
        .cloned()
        .filter_map(|field| field.ident)
        .collect();

    let required_fields_init = {};

    Ok(quote::quote! {
        impl #generics ::partial_config::Partial for #partial_ident #generics {
            type Target = #ident #generics;

            type Error = ::partial_config::Error;

            fn build(self) -> Result<Self::Target, Self::Error> {
                let mut missing_fields = ::std::vec::Vec::new();
                #assembling_config;



                if missing_fields.is_empty() {
                    #error
                } else {
                    Ok(
                        Self::Target {
                            #optional_fields
                        }
                    )
                }
            }

            fn source(self, value: impl ::partial_config::Source<Self::Target>) -> Result<Self, Self::Error> {
                todo!()
            }

            fn override_with(self, other: Self) -> Self {
                todo!()
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
