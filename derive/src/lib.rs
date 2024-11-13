use proc_macro::TokenStream;
use proc_macro_error2::proc_macro_error;
use proc_macro_error2::{OptionExt, ResultExt};
use quote::ToTokens;
use std::collections::{BTreeSet, HashMap};
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, DeriveInput, Field, Generics, Ident, Meta,
};

#[proc_macro_error]
#[proc_macro_derive(
    HasPartial,
    attributes(partial_derives, partial_rename, env_source, env, partial_only)
)]
pub fn has_partial(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        vis,
    } = syn::parse_macro_input!(input as DeriveInput);
    // TODO: support inheriting `pub(crate)
    // TODO: panic on generics

    let partial_ident = partial_struct_name(&ident, &attrs);

    match vis {
        syn::Visibility::Public(_) => {}
        _ => {
            proc_macro_error2::abort!(vis, "Cannot implement `HasPartial` for a private structure.";
                help = "If your structure is private, it is better to convert to it with an `Into::into` rather than directly derive `HasPartial`, which by definition will expose some of the fields"
            )
        }
    };

    let strct = match data {
        syn::Data::Struct(thing) => thing,
        syn::Data::Enum(_) => {
            proc_macro_error2::abort!(
                ident, "Enums are not supported.";
                help = "While it is possible to support `enum`s in principle, this is most likely an X-Y problem. You should use `partial_config` to build your internal `enum` with an extra layer.",
            );
        }
        syn::Data::Union(_) => {
            proc_macro_error2::abort!(
                ident, "Data unions are not supported.";
                help = "Data unions are not usually used in Safe Rust, even though they could be, this is discouraged in favour of Enums, which are not supported either. Consider using a `struct` instead."
            );
        }
    };

    let fields = match strct.fields {
        syn::Fields::Named(namede) => namede.named,
        syn::Fields::Unnamed(flds) => {
            proc_macro_error2::abort!(
                flds, "Unnamed fields can't be named in configuration layers.";
                help = "If the field is unnamed, I cannot find a consistent way of naming them in configuration layers, because they muse be human facing. You are probably applying this derive macro to a tuple structure, which is not a sensible input."
            );
        }
        syn::Fields::Unit => {
            proc_macro_error2::abort!(
                strct.fields, "Unit fields cannot be named.";
                help = "If the field is unnamed, I cannot find a consistent way of naming them in configuration layers. Add a dummy field with e.g. `PhantomData` to silence this error."
            );
        }
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
        .map(|field| Field {
            attrs: field
                .attrs
                .into_iter()
                .filter(|attr| !attr.path().is_ident("env"))
                .map(|attr| {
                    if attr.path().is_ident("partial_only") {
                        let contents: syn::Meta = attr
                            .parse_args()
                            .expect_or_abort("Attribute failed to parse");
                        syn::parse_quote! {
                            #[#contents]
                        }
                    } else {
                        attr
                    }
                })
                .collect(),
            ..field
        })
        .collect();

    // TODO: Forward all other derives unless otherwise specified.
    // Do not remove serde unless required to
    let derives: Vec<Attribute> = attribute_assign(&attrs);

    let output = quote::quote! {
        #(#derives)*
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

fn partial_struct_name(ident: &Ident, attrs: &Vec<Attribute>) -> Ident {
    let mut ident = quote::format_ident!("Partial{}", ident);
    for attr in attrs {
        if attr.path().is_ident("partial_rename") {
            let identifier: Ident = attr
                .parse_args()
                .expect_or_abort("Failed to parse partial_rename identifier");
            ident = identifier;
        }
    }
    ident
}

fn attribute_assign(attrs: &Vec<Attribute>) -> Vec<Attribute> {
    let mut derives: Punctuated<syn::Path, Comma> = Punctuated::new();
    let mut out_attrs: Vec<Attribute> = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("partial_derives") {
            let nested = attr
                .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
                .expect_or_abort("Invalid specification for `partial_derives`");
            for item in nested {
                match item {
                    Meta::Path(pth) =>  {
                        derives.push(pth);
                    },
                    item => proc_macro_error2::abort!(item, "The paths specified must be specific derive macros, e.g. Clone, got {} instead, which is not allowed", item.to_token_stream())
                }
            }
        } else if attr.path().is_ident("partial_only") {
            let contents: syn::Meta = attr
                .parse_args()
                .expect_or_abort("Attributes failed to parse");
            out_attrs.push(syn::parse_quote! {
                #[#contents]
            })
        }
    }

    // TODO: emit warning
    if !derives.iter().any(|thing| thing.is_ident("Default")) {
        derives.push(syn::parse_quote! {Default});
    }
    vec![syn::parse_quote! {
        #[derive(#derives)]
    }]
}

fn impl_partial(
    generics: &Generics,
    ident: &Ident,
    partial_ident: &Ident,
    required_fields: &Punctuated<Field, Comma>,
    optional_fields: &Punctuated<Field, Comma>,
) -> Result<proc_macro2::TokenStream, &'static str> {
    let error: syn::Expr = syn::parse_quote! {
        ::core::result::Result::Err(::partial_config::Error::MissingFields {
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

fn extract_option_generic(ty: &syn::Type) -> syn::Type {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| match &segment.arguments {
                syn::PathArguments::None => {
                    proc_macro_error2::abort!(segment, "The Option does not have any arguments")
                }
                syn::PathArguments::Parenthesized(_) => proc_macro_error2::abort!(
                    segment,
                    "The option cannot have parenthesised arguments"
                ),
                syn::PathArguments::AngleBracketed(generics) => {
                    match generics
                        .args
                        .first()
                        .expect_or_abort("Cannot have an empty set of generic arguments")
                    {
                        syn::GenericArgument::Lifetime(_) => todo!(),
                        syn::GenericArgument::Type(ty) => ty.clone(),
                        syn::GenericArgument::Const(_) => todo!(),
                        syn::GenericArgument::AssocType(_) => todo!(),
                        syn::GenericArgument::AssocConst(_) => todo!(),
                        syn::GenericArgument::Constraint(_) => todo!(),
                        _ => todo!(),
                    }
                }
            })
            .expect_or_abort("Failed to obtain type"),
        _ => todo!("Not implemented yet"),
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

#[proc_macro_error]
#[proc_macro_derive(EnvSourced, attributes(env_var_rename, env))]
pub fn env_sourced(input: TokenStream) -> TokenStream {
    let DeriveInput {
        data,
        attrs,
        ident: in_ident,
        ..
    } = syn::parse_macro_input!(input as DeriveInput);

    let out_ident: Ident = env_var_struct_name(attrs);
    let strct = match data {
        syn::Data::Struct(strct) => strct,
        syn::Data::Enum(_) => panic!("Enums are not supported"),
        syn::Data::Union(_) => panic!("Data unions are not supported"),
    };

    let fields: Punctuated<Field, Comma> = match strct.fields {
        syn::Fields::Named(fld) => fld.named,
        _ => unreachable!(),
    };

    let EnvVarFieldsResult {
        fields: all_fields,
        default_mappings,
    } = env_var_fields(&fields);

    let default_struct = impl_default_env(default_mappings);
    let impl_source = impl_source(&fields);

    let output = quote::quote! {
    pub struct #out_ident<'a> {
        #all_fields
    }

    impl<'a> ::partial_config::env::EnvSourced<'a> for #in_ident {
        type Source = #out_ident<'a>;
    }

    impl<'a> #out_ident<'a> {
        pub const fn new() -> Self {
            #default_struct
        }
    }

    impl<'a> Default for #out_ident<'a> {
        fn default() -> Self {
            #default_struct
        }
    }

    impl<'a> ::partial_config::Source<#in_ident> for #out_ident<'a> {
        type Error = ::partial_config::Error;

        fn to_partial(self) -> Result<<#in_ident as ::partial_config::HasPartial>::Partial, Self::Error> {
            pub type Issue86935Workaround = <#in_ident as ::partial_config::HasPartial>::Partial;

            Ok(Issue86935Workaround {
                #impl_source
            })
        }

        fn name(&self) -> String {
            "Environment Variables".to_owned()
        }
    }
    };
    TokenStream::from(output)
}

struct EnvVarFieldsResult {
    fields: Punctuated<Field, Comma>,
    default_mappings: HashMap<Ident, BTreeSet<Ident>>,
}

fn is_string(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(pth) => pth.path.is_ident("String") || pth.path.is_ident("str"),
        syn::Type::Reference(reference) => is_string(&reference.elem),
        _ => false,
    }
}

fn impl_source(fields: &Punctuated<Field, Comma>) -> Punctuated<syn::FieldValue, Comma> {
    fields
        .iter()
        .map(|Field { ident, ty, .. }| -> syn::FieldValue {
            if let Some(ident) = ident {
                if is_string(&ty) {
                    syn::parse_quote! {
                        #ident: ::partial_config::env::extract(&self.#ident)?
                    }
                } else {
                    let inner_ty = if is_option(ty) {
                        extract_option_generic(ty)
                    } else {
                        ty.clone()
                    };
                    syn::parse_quote! {
                        #ident: ::partial_config::env::extract(&self.#ident)?
                        .map(|s: String| <#inner_ty as ::core::str::FromStr>::from_str(&s))
                        .transpose()
                        .map_err(|e|
                            ::partial_config::Error::ParseFieldError {
                                field_name: stringify!(#ident),
                                field_type: stringify!(#ty),
                                error_condition: Box::new(e)
                            })?
                    }
                }
            } else {
                proc_macro_error2::abort!(ident, "Non-struct like fields are not allowed");
            }
        })
        .collect()
}

fn impl_default_env(default_mappings: HashMap<Ident, BTreeSet<Ident>>) -> syn::ExprStruct {
    let elements: Punctuated<syn::FieldValue, Comma> = default_mappings
        .iter()
        .map(|(field_name, env_var_strings)| -> syn::FieldValue {
            let env_var_strings: Punctuated<syn::LitStr, Comma> = env_var_strings
                .iter()
                .cloned()
                .map(|ident| -> syn::LitStr {
                    syn::LitStr::new(&ident.to_string(), proc_macro2::Span::call_site())
                })
                .collect();
            syn::parse_quote! {
                #field_name: [#env_var_strings]
            }
        })
        .collect();

    syn::parse_quote! {
        Self {
            #elements
        }
    }
}

fn env_var_fields(fields: &Punctuated<Field, Comma>) -> EnvVarFieldsResult {
    let mut output = Punctuated::new();
    let mut default_mappings: HashMap<Ident, BTreeSet<Ident>> = HashMap::new();
    for field in fields {
        let mut n = 0_usize;
        field.attrs.iter().for_each(|attr| {
            if attr.path().is_ident("env") {
                let nested = attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated).expect_or_abort("Invalid specification for the `env` attribute");
                let env_vars: BTreeSet<Ident> = nested.iter().
                    filter_map(|item| {
                        match item {
                            Meta::Path(pth) => Some(pth.get_ident().expect_or_abort("Must have identifier and not a path").clone()),
                            _ => None
                        }
                    })
                    .collect();
                n+=env_vars.len();
                let key = field.ident.clone().expect_or_abort("Identifiers for all fields must be known at this point");
                default_mappings.entry(key.clone())
                    .and_modify(|previous| {
                        if !previous.is_disjoint(&env_vars) {
                            proc_macro_error2::emit_error!(key, "Environment variable specifications must be disjoint. The field {key} has the following duplicate specifications {:?}",
                                previous.intersection(&env_vars).map(|ident| ident.to_string()).collect::<Vec<_>>());
                        }
                        previous.extend(env_vars.iter().cloned())
                    })
                    .or_insert(env_vars);
            }
        });
        if n == 0 {
            proc_macro_error2::emit_error!(field.ident, "At least one `env` directive must be specified";
                help = "Try using an uppercase version of the field name: {}", field.ident.to_token_stream().to_string().to_uppercase();
                note = "It is better to enforce that all env-var deserializeable fields are explicitly set in the code.")
        }
        // TODO: check uniqueness in leaf nodes
        // TODO: Check for empty nodes and replace with uppercase
        let ty: syn::Type = syn::parse_quote! {
            [&'a str; #n]
        };

        output.push(Field {
            ty,
            attrs: vec![],
            ..field.clone()
        });
    }

    EnvVarFieldsResult {
        fields: output,
        default_mappings,
    }
}

fn env_var_struct_name(attrs: Vec<Attribute>) -> Ident {
    let mut ident = syn::parse_quote! { EnvVarSource };
    for attr in attrs {
        if attr.path().is_ident("env_var_rename") {
            let identifier: Ident = attr
                .parse_args()
                .expect_or_abort("Failed to parse env_var_rename identifier. ");
            ident = identifier;
        }
    }
    ident
}
