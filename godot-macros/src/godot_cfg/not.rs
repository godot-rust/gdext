use super::*;
use proc_macro2::Group;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationNot(pub(crate) GodotConfigurationPredicate);

impl TryFrom<Group> for GodotConfigurationNot {
    type Error = GodotConditionCompilationError;

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
            n => Err(Self::Error::IncorrectNumberOfNotArguments(n, group)),
        }
    }
}

impl GodotConditionalCompilation for GodotConfigurationNot {
    fn should_compile(&self) -> bool {
        !self.0.should_compile()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

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
                GodotConfigurationPredicate::Option(GodotConfigurationOption::Test)
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
}
