use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

mod derive_godot_class;
mod util;

#[proc_macro_derive(GodotClass, attributes(godot, property))]
pub fn derive_native_class(input: TokenStream) -> TokenStream {
    let input2 = TokenStream2::from(input);
    let result2: TokenStream2 = match derive_godot_class::derive_godot_class(input2) {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    };
    TokenStream::from(result2)
}

#[proc_macro_attribute]
pub fn godot(meta: TokenStream, input: TokenStream) -> TokenStream {
    todo!()
}
