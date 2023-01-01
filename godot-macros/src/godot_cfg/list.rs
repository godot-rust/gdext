use super::*;
use proc_macro2::{Group, TokenTree};
use std::ops::Deref;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(super) struct GodotConfigurationPredicateList(pub(crate) Vec<GodotConfigurationPredicate>);

impl Deref for GodotConfigurationPredicateList {
    type Target = Vec<GodotConfigurationPredicate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Group> for GodotConfigurationPredicateList {
    type Error = GodotConditionCompilationError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        let mut inner = vec![];
        let mut iter = group.stream().into_iter().peekable();
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
                            return Err(Self::Error::ListUnexpectedTokenInGroup(group, tt.clone()))
                        }
                    }
                }
                // Anything else
                Some(tt) => return Err(Self::Error::ListUnexpectedTokenInGroup(group, tt)),
            }
        }
        Ok(Self(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
