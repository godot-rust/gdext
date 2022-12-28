use crate::godot_cfg::{GodotConditionalCompilationError, GodotConfigurationPredicate};
use proc_macro2::{Group, TokenTree};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub enum GodotConfigurationPredicateListError {
    UnexpectedTokenInGroup(Group, TokenTree),
    GodotConditionalCompilationError(GodotConditionalCompilationError),
}

impl Display for GodotConfigurationPredicateListError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ve: venial::Error = self.clone().into();
        write!(f, "{}", ve)
    }
}

impl Error for GodotConfigurationPredicateListError {}

impl Into<venial::Error> for GodotConfigurationPredicateListError {
    fn into(self) -> venial::Error {
        match self {
            Self::UnexpectedTokenInGroup(group, tt) => {
                let message = format!("Unexpected token found in group: {}, {}", &group, &tt);
                venial::Error::new_at_tokens(group, message)
            }
            Self::GodotConditionalCompilationError(error) => venial::Error::new(error.to_string()),
        }
    }
}

impl From<GodotConditionalCompilationError> for GodotConfigurationPredicateListError {
    fn from(error: GodotConditionalCompilationError) -> Self {
        Self::GodotConditionalCompilationError(error)
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationPredicateList(pub(crate) Vec<GodotConfigurationPredicate>);

impl Deref for GodotConfigurationPredicateList {
    type Target = Vec<GodotConfigurationPredicate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Group> for GodotConfigurationPredicateList {
    type Error = GodotConfigurationPredicateListError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        let mut inner = vec![];
        let mut iter = group.clone().stream().into_iter().peekable();
        loop {
            let next = iter.next();
            if next.is_none() {
                break;
            }
            let first_token = next.unwrap();

            match iter.next() {
                // Single word
                None | Some(TokenTree::Punct(_)) => {
                    inner.push(GodotConfigurationPredicate::try_from(first_token)?)
                }
                // Word followed by group
                Some(TokenTree::Group(group)) => {
                    match iter.peek() {
                        // Group is last token or followed by punctuation
                        None | Some(TokenTree::Punct(_)) => {
                            let _ = iter.next(); // Skip the punctuation
                            inner.push(GodotConfigurationPredicate::try_from((
                                first_token,
                                group.into(),
                            ))?)
                        }
                        // Anything else
                        Some(tt) => {
                            return Err(Self::Error::UnexpectedTokenInGroup(group, tt.clone()))
                        }
                    }
                }
                // Anything else
                Some(tt) => return Err(Self::Error::UnexpectedTokenInGroup(group, tt.clone())),
            }
        }
        Ok(Self(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::godot_cfg::option::GodotConfigurationOption;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    #[test]
    fn test_predicate_list() {
        let ts = TokenStream::from_str("(test, doctest)").unwrap();
        let tt = ts.into_iter().next().unwrap();
        if let TokenTree::Group(group) = tt {
            let list = GodotConfigurationPredicateList::try_from(group).unwrap();
            assert_eq!(
                list,
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::DocTest),
                ])
            )
        } else {
            panic!("Token tree was not a group")
        }
    }
}
