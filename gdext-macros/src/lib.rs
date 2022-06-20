use proc_macro::TokenStream;

mod derive_godot_class;
mod util;

#[proc_macro_derive(NativeClass, attributes(godot, property))]
pub fn derive_native_class(input: TokenStream) -> TokenStream {
    let input2 = input.into();
    match derive_godot_class::derive_native_class(input2) {
        Ok(output) => output.into(),
        Err(error) => to_compile_errors(error),
    }
}

#[proc_macro_attribute]
pub fn godot(meta: TokenStream, input: TokenStream) -> TokenStream {
    todo!()
}

fn to_compile_errors(error: venial::Error) -> proc_macro2::TokenStream {
    let compile_error = error.to_compile_error();
    //quote!{ #compile_error }
    compile_error
}
