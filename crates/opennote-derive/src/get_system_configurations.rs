use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result, parse};

pub(crate) fn derive(input: TokenStream) -> Result<proc_macro2::TokenStream> {
    let input: DeriveInput = parse(input)?;

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics crate::traits::GetSystemConfigurations for #name #type_generics #where_clause
        {
            fn get_system_configurations(&self) -> &crate::configurations::system::SystemConfigurations {
                &self.system_configurations
            }
        }
    };

    Ok(expanded)
}
