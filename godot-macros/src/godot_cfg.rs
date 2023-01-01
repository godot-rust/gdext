mod all;
mod any;
mod error;
mod list;
mod not;
mod option;
mod predicate;

use all::*;
use any::*;
use error::*;
use list::*;
use not::*;
use option::*;
use predicate::*;

trait GodotConditionalCompilation {
    fn should_compile(&self) -> bool;
}

pub fn should_compile(ts: proc_macro2::TokenStream) -> Result<bool, venial::Error> {
    let predicate = GodotConfigurationPredicate::try_from(ts)?;
    Ok(predicate.should_compile())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    #[test]
    fn test_should_compile() {
        let ts = TokenStream::from_str("any(all(test, not(doctest)), doctest)").unwrap();
        assert!(should_compile(ts).unwrap());
    }
}
