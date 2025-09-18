use quote::quote;

pub fn generate_tuple_implementations(
    name: &syn::Ident,
    fields_unnamed: &syn::FieldsUnnamed,
) -> proc_macro2::TokenStream {
    if fields_unnamed.unnamed.len() != 1 {
        panic!(
            "Protto only supports tuple structs with exactly one field, found {}",
            fields_unnamed.unnamed.len()
        );
    }

    let inner_type = &fields_unnamed.unnamed[0].ty;

    quote! {
        impl From<#inner_type> for #name {
            fn from(value: #inner_type) -> Self {
                #name(value)
            }
        }

        impl From<#name> for #inner_type {
            fn from(my: #name) -> Self {
                my.0
            }
        }
    }
}
