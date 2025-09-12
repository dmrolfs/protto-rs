use quote::quote;
use crate::debug::CallStackDebug;
use crate::analysis::{
    attribute_parser,
    field_analysis::FieldProcessingContext,
};
use crate::struct_impl;
use crate::field::field_processor;
use crate::error_handler;
use crate::debug;

pub struct StructImplConfig<'a> {
    pub name: &'a syn::Ident,
    pub fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    pub proto_module: &'a str,
    pub proto_name: &'a str,
    pub proto_path: &'a syn::Path,
    pub struct_level_error_type: &'a Option<syn::Type>,
    pub struct_level_error_fn: &'a Option<String>,
}

pub fn generate_struct_implementations_with_migration(
    config: struct_impl::StructImplConfig,
) -> proc_macro2::TokenStream {
    let struct_name = config.name;
    let fields = config.fields;

    // Generate bidirectional conversions in single pass
    let mut field_conversions = Vec::new();
    let mut conversion_errors = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();

        let _trace = CallStackDebug::with_context(
            "generate_struct_implementations_with_migration",
            config.name,
            field_name,
            &[],
        );

        let error_name = syn::Ident::new(
            &format!("{}ConversionError", struct_name),
            proc_macro2::Span::call_site(),
        );

        let ctx = FieldProcessingContext::new(
            struct_name,
            field,
            &error_name,
            config.struct_level_error_type,
            config.struct_level_error_fn,
            config.proto_module,
            config.proto_name,
        );

        match field_processor::generate_bidirectional_field_conversion(field, &ctx) {
            Ok((proto_to_rust, rust_to_proto)) => {
                field_conversions.push((field_name, proto_to_rust, rust_to_proto));
            }
            Err(error_msg) => {
                conversion_errors.push((field_name, error_msg));
            }
        }
    }

    // Handle any conversion errors
    if !conversion_errors.is_empty() {
        let error_msgs: Vec<String> = conversion_errors
            .iter()
            .map(|(field_name, error)| format!("Field '{}': {}", field_name, error))
            .collect();
        let combined_error = error_msgs.join("\n");
        return quote! { compile_error!(#combined_error); };
    }

    // Generate From and Into implementations
    let proto_to_rust_fields: Vec<_> = field_conversions
        .iter()
        .map(|(_, proto_to_rust, _)| proto_to_rust)
        .collect();
    let rust_to_proto_fields: Vec<_> = field_conversions
        .iter()
        .map(|(_, _, rust_to_proto)| rust_to_proto)
        .collect();

    let proto_type_path = format!("{}::{}", config.proto_module, config.proto_name);
    let proto_type: syn::Path = syn::parse_str(&proto_type_path).unwrap();

    quote! {
        impl From<#proto_type> for #struct_name {
            fn from(proto_struct: #proto_type) -> Self {
                Self {
                    #(#proto_to_rust_fields),*
                }
            }
        }

        impl Into<#proto_type> for #struct_name {
            fn into(self) -> #proto_type {
                let my_struct = self;
                #proto_type {
                    #(#rust_to_proto_fields),*
                }
            }
        }
    }
}

pub fn generate_struct_implementations(config: StructImplConfig) -> proc_macro2::TokenStream {
    let error_name = error_handler::default_error_name(config.name);

    let (conversion_error_def, error_conversions, needs_try_from) =
        error_handler::generate_error_definitions_if_needed(
            config.name,
            config.fields,
            config.struct_level_error_type,
        );

    let actual_error_type = error_handler::get_actual_error_type(
        needs_try_from,
        config.struct_level_error_type,
        &error_name,
    );

    //todo: refactor to perform a single interation on fields w dual generation
    let from_proto_fields: Vec<_> =
        generate_from_proto_fields(config.fields, &config, &error_name).collect();
    let from_my_fields: Vec<_> =
        generate_from_my_fields(config.fields, &config, &error_name).collect();

    let name = config.name;
    let proto_path = config.proto_path;

    let final_impl = if needs_try_from {
        quote! {
            #conversion_error_def
            #error_conversions

            impl TryFrom<#proto_path> for #name {
                type Error = #actual_error_type;

                fn try_from(proto_struct: #proto_path) -> Result<Self, Self::Error> {
                    Ok(Self {
                        #(#from_proto_fields),*
                    })
                }
            }

            impl From<#name> for #proto_path {
                fn from(my_struct: #name) -> Self {
                    Self {
                        #(#from_my_fields),*
                    }
                }
            }
        }
    } else {
        quote! {
            impl From<#proto_path> for #name {
                fn from(proto_struct: #proto_path) -> Self {
                    Self {
                        #(#from_proto_fields),*
                    }
                }
            }

            impl From<#name> for #proto_path {
                fn from(my_struct: #name) -> Self {
                    Self {
                        #(#from_my_fields),*
                    }
                }
            }
        }
    };

    debug::debug_struct_conversion_generation(
        name,
        "FINAL_GENERATION",
        &quote! {
            Self {
                #(#from_proto_fields),*
            }
        },
        &quote! {
            Self {
                #(#from_my_fields),*
            }
        },
        &quote!(#final_impl),
        &[
            ("total_fields", config.fields.len().to_string()),
            ("proto_module", config.proto_module.to_string()),
            ("proto_name", config.proto_name.to_string()),
            ("needs_try_from", needs_try_from.to_string()),
            ("error_type", quote!(#actual_error_type).to_string()),
        ],
    );

    final_impl
}

fn generate_from_proto_fields<'a>(
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    config: &'a StructImplConfig<'a>,
    error_name: &'a syn::Ident,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    fields.iter().map(move |field| {
        let ctx = FieldProcessingContext::new(
            config.name,
            field,
            error_name,
            config.struct_level_error_type,
            config.struct_level_error_fn,
            config.proto_module,
            config.proto_name,
        );

        field_processor::generate_from_proto_field(field, &ctx)
    })
}

fn generate_from_my_fields<'a>(
    fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    config: &'a StructImplConfig<'a>,
    error_name: &'a syn::Ident,
) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
    fields
        .iter()
        .filter(|field| !attribute_parser::has_proto_ignore(field))
        .map(move |field| {
            let ctx = FieldProcessingContext::new(
                config.name,
                field,
                error_name,
                config.struct_level_error_type,
                config.struct_level_error_fn,
                config.proto_module,
                config.proto_name,
            );

            field_processor::generate_from_my_field(field, &ctx)
        })
}
