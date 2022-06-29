// Note: some code duplication with codegen crate

use proc_macro2::{Ident, Literal};
use quote::format_ident;
use quote::spanned::Spanned;
use venial::Error;

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

// pub fn strlit(s: &str) -> Literal {
//     Literal::string(s)
// }

pub fn bail<R, T>(msg: &str, tokens: T) -> Result<R, Error>
where
    T: Spanned,
{
    Err(Error::new_at_span(tokens.__span(), msg))
}
