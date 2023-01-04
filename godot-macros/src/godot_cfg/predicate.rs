use super::*;
use proc_macro2::{Group, Ident, TokenStream, TokenTree};

// ConfigurationPredicate doesn't seem to be accessible so we'll make our own.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(super) enum GodotConfigurationPredicate {
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
    type Error = GodotConditionCompilationError;

    fn try_from(ident: &Ident) -> Result<Self, Self::Error> {
        Ok(GodotConfigurationOption::try_from(ident)?.into())
    }
}

impl TryFrom<TokenTree> for GodotConfigurationPredicate {
    type Error = GodotConditionCompilationError;

    fn try_from(tt: TokenTree) -> Result<Self, Self::Error> {
        match tt {
            TokenTree::Ident(ident) => Self::try_from(&ident),
            TokenTree::Group(group) => Self::try_from(group.stream()),
            _ => Err(Self::Error::PredicateExpectedIdentGotOther(tt)),
        }
    }
}

impl TryFrom<(Ident, Group)> for GodotConfigurationPredicate {
    type Error = GodotConditionCompilationError;

    fn try_from((ident, group): (Ident, Group)) -> Result<Self, Self::Error> {
        match ident.to_string().as_str() {
            "any" => Ok(GodotConfigurationAny::try_from(group)?.into()),
            "all" => Ok(GodotConfigurationAll::try_from(group)?.into()),
            "not" => Ok(GodotConfigurationNot::try_from(group)?.into()),
            _ => Err(Self::Error::PredicateInvalidGrouping(ident, group)),
        }
    }
}

impl TryFrom<(TokenTree, TokenTree)> for GodotConfigurationPredicate {
    type Error = GodotConditionCompilationError;

    fn try_from(tokens: (TokenTree, TokenTree)) -> Result<Self, Self::Error> {
        match tokens {
            (TokenTree::Ident(ident), TokenTree::Group(group)) => Self::try_from((ident, group)),
            (t1, t2) => Err(Self::Error::PredicateInvalidAdjacentTokens(t1, t2)),
        }
    }
}

impl TryFrom<TokenStream> for GodotConfigurationPredicate {
    type Error = GodotConditionCompilationError;

    fn try_from(ts: TokenStream) -> Result<Self, Self::Error> {
        let tokens: Vec<_> = ts.clone().into_iter().collect();
        match tokens.len() {
            1 => {
                let token = tokens.into_iter().next().unwrap();
                Ok(Self::try_from(token)?)
            }
            2 => {
                let mut iter = tokens.into_iter();
                let token1 = iter.next().unwrap();
                let token2 = iter.next().unwrap();
                Ok(Self::try_from((token1, token2))?)
            }
            _ => Err(Self::Error::PredicateInvalidFormat(ts)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
