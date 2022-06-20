// Note: some code duplication with codegen crate

use proc_macro2::{Ident, Literal};
use quote::format_ident;

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn strlit(s: &str) -> Literal {
    Literal::string(s)
}
