use crate::godot_cfg::all::*;
use crate::godot_cfg::any::*;
use crate::godot_cfg::list::*;
use crate::godot_cfg::not::*;
use crate::godot_cfg::option::*;
use proc_macro2::{Group, Ident, TokenTree};
use std::error::Error;
use std::fmt::{Display, Formatter};

mod all;
mod any;
mod list;
mod not;
mod option;

#[derive(Debug, Clone)]
pub enum GodotConditionalCompilationError {
    UnableToParse(String),
    VenialError(venial::Error),
}

impl From<GodotConfigurationOptionError> for GodotConditionalCompilationError {
    fn from(value: GodotConfigurationOptionError) -> Self {
        Self::VenialError(value.into())
    }
}

impl From<GodotConfigurationPredicateListError> for GodotConditionalCompilationError {
    fn from(value: GodotConfigurationPredicateListError) -> Self {
        Self::VenialError(value.into())
    }
}

impl Display for GodotConditionalCompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnableToParse(e) => {
                write!(f, "Unable to parse '{}'", e)
            }
            Self::VenialError(e) => write!(f, "{}", e),
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

impl From<GodotConfigurationOption> for GodotConfigurationPredicate {
    fn from(option: GodotConfigurationOption) -> Self {
        Self::Option(option)
    }
}

impl From<GodotConfigurationAll> for GodotConfigurationPredicate {
    fn from(all: GodotConfigurationAll) -> Self {
        Self::All(Box::new(all))
    }
}

impl From<GodotConfigurationAny> for GodotConfigurationPredicate {
    fn from(any: GodotConfigurationAny) -> Self {
        Self::Any(Box::new(any))
    }
}

impl From<GodotConfigurationNot> for GodotConfigurationPredicate {
    fn from(not: GodotConfigurationNot) -> Self {
        Self::Not(Box::new(not))
    }
}

impl TryFrom<&Ident> for GodotConfigurationPredicate {
    type Error = GodotConditionalCompilationError;

    fn try_from(ident: &Ident) -> Result<Self, Self::Error> {
        Ok(GodotConfigurationOption::try_from(ident)?.into())
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
            "any" => Ok(GodotConfigurationAny::try_from(group)?.into()),
            "all" => Ok(GodotConfigurationAll::try_from(group)?.into()),
            "not" => Ok(GodotConfigurationNot::try_from(group)?.into()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

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
                                GodotConfigurationPredicate::Option(
                                    GodotConfigurationOption::DocTest
                                )
                            ))),
                        ])
                    ))),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::DocTest),
                ])
            )))
        )
    }
}
