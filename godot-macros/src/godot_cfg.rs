use proc_macro2::{Group, Ident, TokenTree};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use crate::godot_cfg::option::{GodotConfigurationOption, GodotConfigurationOptionError};

mod option;

#[derive(Debug)]
pub enum GodotConditionalCompilationError {
    UnableToParse(String),
    VenialError(venial::Error),
}

impl From<GodotConfigurationOptionError> for GodotConditionalCompilationError {
    fn from(value: GodotConfigurationOptionError) -> Self {
        Self::VenialError(value.into())
    }
}

impl Display for GodotConditionalCompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnableToParse(e) => {
                write!(f, "Unable to parse '{}'", e)
            }
            Self::VenialError(e) => write!(f, "{}", e)
        }
    }
}

impl Error for GodotConditionalCompilationError {}

pub trait GodotConditionalCompilation {
    fn should_compile(&self) -> bool;
}

// ConfigurationPredicate doesn't seem to be accessible so we'll make our own.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum GodotConfigurationPredicate {
    Option(GodotConfigurationOption),
    All(Box<GodotConfigurationAll>),
    Any(Box<GodotConfigurationAny>),
    Not(Box<GodotConfigurationNot>),
}

impl GodotConditionalCompilation for GodotConfigurationPredicate {
    fn should_compile(&self) -> bool {
        match self {
            Self::Option(c) => c.should_compile(),
            Self::All(c) => c.should_compile(),
            Self::Any(c) => c.should_compile(),
            Self::Not(c) => c.should_compile(),
        }
    }
}

impl TryFrom<&Ident> for GodotConfigurationPredicate {
    type Error = GodotConditionalCompilationError;

    fn try_from(ident: &Ident) -> Result<Self, Self::Error> {
        Ok(Self::Option(GodotConfigurationOption::try_from(ident)?))
    }
}

impl TryFrom<TokenTree> for GodotConfigurationPredicate {
    type Error = GodotConditionalCompilationError;

    fn try_from(tt: TokenTree) -> Result<Self, Self::Error> {
        match tt {
            TokenTree::Ident(ident) => Self::try_from(&ident),
            _ => Err(Self::Error::UnableToParse(tt.to_string())),
        }
    }
}

impl TryFrom<(Ident, Group)> for GodotConfigurationPredicate {
    type Error = GodotConditionalCompilationError;

    fn try_from((ident, group): (Ident, Group)) -> Result<Self, Self::Error> {
        match ident.to_string().as_str() {
            "any" => Ok(Self::Any(Box::new(GodotConfigurationAny::try_from(group)?))),
            "all" => Ok(Self::All(Box::new(GodotConfigurationAll::try_from(group)?))),
            "not" => Ok(Self::Not(Box::new(GodotConfigurationNot::try_from(group)?))),
            _ => Err(Self::Error::UnableToParse(format!(
                "unrecognised ident: {}",
                ident
            ))),
        }
    }
}

impl TryFrom<(TokenTree, TokenTree)> for GodotConfigurationPredicate {
    type Error = GodotConditionalCompilationError;

    fn try_from(tokens: (TokenTree, TokenTree)) -> Result<Self, Self::Error> {
        match tokens {
            (TokenTree::Ident(ident), TokenTree::Group(group)) => Self::try_from((ident, group)),
            _ => Err(Self::Error::UnableToParse(format!(
                "Invalid predicate format: '{}{}'",
                tokens.0.to_string(),
                tokens.1.to_string()
            ))),
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationAll(GodotConfigurationPredicateList);

impl TryFrom<Group> for GodotConfigurationAll {
    type Error = GodotConditionalCompilationError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        Ok(Self(GodotConfigurationPredicateList::try_from(group)?))
    }
}

impl GodotConditionalCompilation for GodotConfigurationAll {
    fn should_compile(&self) -> bool {
        self.0.iter().all(|option| option.should_compile())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationAny(GodotConfigurationPredicateList);

impl TryFrom<Group> for GodotConfigurationAny {
    type Error = GodotConditionalCompilationError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        Ok(Self(GodotConfigurationPredicateList::try_from(group)?))
    }
}

impl GodotConditionalCompilation for GodotConfigurationAny {
    fn should_compile(&self) -> bool {
        self.0.iter().any(|option| option.should_compile())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationNot(GodotConfigurationPredicate);

impl TryFrom<Group> for GodotConfigurationNot {
    type Error = GodotConditionalCompilationError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        let tokens: Vec<_> = group.stream().into_iter().collect();
        match tokens.len() {
            1 => {
                // ToDo: Make this more efficient
                let mut iter = tokens.into_iter();
                let likely_ident = iter.next().unwrap();
                Ok(Self(GodotConfigurationPredicate::try_from(likely_ident)?))
            }
            2 => {
                // ToDo: Make this more efficient
                let mut iter = tokens.into_iter();
                let likely_ident = iter.next().unwrap();
                let likely_group = iter.next().unwrap();
                Ok(Self(GodotConfigurationPredicate::try_from((
                    likely_ident,
                    likely_group,
                ))?))
            }
            n => Err(Self::Error::UnableToParse(format!(
                "not may only have one predicate inside it, found {}: {}",
                n, group
            ))),
        }
    }
}

impl GodotConditionalCompilation for GodotConfigurationNot {
    fn should_compile(&self) -> bool {
        !self.0.should_compile()
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationPredicateList(Vec<GodotConfigurationPredicate>);

impl Deref for GodotConfigurationPredicateList {
    type Target = Vec<GodotConfigurationPredicate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Group> for GodotConfigurationPredicateList {
    type Error = GodotConditionalCompilationError;

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
                        _ => return Err(Self::Error::UnableToParse(group.to_string())),
                    }
                }
                // Anything else
                _ => return Err(Self::Error::UnableToParse(group.to_string())),
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
                    GodotConfigurationPredicate::Option(GodotConfigurationOption ::Test),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption ::DocTest),
                ])
            )
        } else {
            panic!("Token tree was not a group")
        }
    }

    #[test]
    fn test_all() {
        let mut ts = TokenStream::from_str("all(test, test)")
            .unwrap()
            .into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::All(Box::new(GodotConfigurationAll(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                ])
            )))
        );

        let mut ts = TokenStream::from_str("all(test, doctest)")
            .unwrap()
            .into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::All(Box::new(GodotConfigurationAll(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption ::DocTest),
                ])
            )))
        );
    }

    #[test]
    fn test_any() {
        let mut ts = TokenStream::from_str("any(test, doctest)")
            .unwrap()
            .into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Any(Box::new(GodotConfigurationAny(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption ::Test),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::DocTest),
                ])
            )))
        );

        let mut ts = TokenStream::from_str("any(doctest)").unwrap().into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Any(Box::new(GodotConfigurationAny(
                GodotConfigurationPredicateList(vec![GodotConfigurationPredicate::Option(
                    GodotConfigurationOption::DocTest),])
            )))
        );
    }

    #[test]
    fn test_not() {
        let mut ts = TokenStream::from_str("not(test)").unwrap().into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                GodotConfigurationPredicate::Option(GodotConfigurationOption ::Test)
            )))
        );

        let mut ts = TokenStream::from_str("not(not(test))").unwrap().into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::Test)
                )))
            )))
        );
    }

    #[test]
    fn test_everything() {
        let mut ts = TokenStream::from_str("any(all(test, not(doctest)), doctest)")
            .unwrap()
            .into_iter();
        let ident = ts.next().unwrap();
        let group = ts.next().unwrap();
        let predicate = GodotConfigurationPredicate::try_from((ident, group)).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Any(Box::new(GodotConfigurationAny(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::All(Box::new(GodotConfigurationAll(
                        GodotConfigurationPredicateList(vec![
                            GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                            GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                                GodotConfigurationPredicate::Option(GodotConfigurationOption::DocTest)
                            ))),
                        ])
                    ))),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::DocTest),
                ])
            )))
        )
    }
}
