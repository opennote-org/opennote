use proc_macro::TokenStream;

mod get_system_configurations;

#[proc_macro_derive(GetSystemConfigurations)]
pub fn derive_get_system_configuration(input: TokenStream) -> TokenStream {
    match get_system_configurations::derive(input) {
        Ok(result) => result.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
