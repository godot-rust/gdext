use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub(super) enum GodotConditionCompilationError {
    ListUnexpectedTokenInGroup(Group, TokenTree),
    OptionUnsupported(Ident),
    OptionInvalidToken(TokenTree),
    PredicateExpectedIdentGotOther(TokenTree),
    PredicateInvalidGrouping(Ident, Group),
    PredicateInvalidAdjacentTokens(TokenTree, TokenTree),
    PredicateInvalidFormat(TokenStream),
    IncorrectNumberOfNotArguments(usize, Group),
}

impl From<GodotConditionCompilationError> for venial::Error {
    fn from(e: GodotConditionCompilationError) -> Self {
        match e {
            GodotConditionCompilationError::ListUnexpectedTokenInGroup(group, tt) => {
                let message = format!("Expected ident found in list '{}, {}'", group, tt);
                venial::Error::new_at_tokens(group, message)
            }
            GodotConditionCompilationError::OptionUnsupported(ident) => {
                let message = format!("Conditional compilation option not supported '{}'", ident);
                venial::Error::new_at_tokens(ident, message)
            }
            GodotConditionCompilationError::OptionInvalidToken(tt) => {
                let message = format!("Invalid token in conditional compilation option '{}'", tt);
                venial::Error::new_at_tokens(tt, message)
            }
            GodotConditionCompilationError::PredicateExpectedIdentGotOther(tt) => {
                let message = format!(
                    "Unexpected token in conditional compilation predicate '{}'",
                    tt
                );
                venial::Error::new_at_tokens(tt, message)
            }
            GodotConditionCompilationError::PredicateInvalidGrouping(ident, group) => {
                let message = format!(
                    "Invalid grouping in conditional compilation predicate '{}{}'",
                    ident, group
                );
                venial::Error::new_at_tokens(ident, message)
            }
            GodotConditionCompilationError::PredicateInvalidAdjacentTokens(tt1, tt2) => {
                let message = format!(
                    "Invalid adjacent tokens in conditional compilation predicate '{}{}'",
                    tt1, tt2
                );
                venial::Error::new_at_tokens(tt1, message)
            }
            GodotConditionCompilationError::PredicateInvalidFormat(ts) => {
                let message = format!("Predicate format invalid '{}'", ts);
                venial::Error::new_at_tokens(ts, message)
            }
            GodotConditionCompilationError::IncorrectNumberOfNotArguments(n, group) => {
                if n == 0 {
                    let message = "There must be one argument inside not()".to_string();
                    venial::Error::new_at_tokens(group, message)
                } else {
                    let message = format!("There must be only one argument inside not{}", group);
                    venial::Error::new_at_tokens(group, message)
                }
            }
        }
    }
}

impl Display for GodotConditionCompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ve: venial::Error = self.clone().into();
        write!(f, "{}", ve)
    }
}

impl Error for GodotConditionCompilationError {}
