use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

mod derive_godot_class;
mod godot_api;
mod itest;
mod util;

#[proc_macro_derive(GodotClass, attributes(godot, property, export, base))]
pub fn derive_native_class(input: TokenStream) -> TokenStream {
    translate(input, derive_godot_class::transform)
}

#[proc_macro_attribute]
pub fn godot_api(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, godot_api::transform)
}

/// Similar to `#[test]`, but runs an integration test with Godot.
///
/// Transforms the `fn` into one returning `bool` (success of the test), which must be called explicitly.
#[proc_macro_attribute]
pub fn itest(_meta: TokenStream, input: TokenStream) -> TokenStream {
    translate(input, itest::transform)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

type ParseResult<T> = Result<T, venial::Error>;

fn translate<F>(input: TokenStream, transform: F) -> TokenStream
where
    F: FnOnce(TokenStream2) -> Result<TokenStream2, venial::Error>,
{
    let input2 = TokenStream2::from(input);
    let result2: TokenStream2 = match transform(input2) {
        Ok(output) => output,
        Err(error) => error.to_compile_error(),
    };
    TokenStream::from(result2)
}
