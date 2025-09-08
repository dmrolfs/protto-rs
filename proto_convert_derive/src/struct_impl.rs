use super::*;
use field_analysis::FieldProcessingContext;

pub struct StructImplConfig<'a> {
    pub name: &'a syn::Ident,
    pub fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    pub proto_module: &'a str,
    pub proto_name: &'a str,
    pub proto_path: &'a syn::Path,
    pub struct_level_error_type: &'a Option<syn::Type>,
    pub struct_level_error_fn: &'a Option<String>,
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
