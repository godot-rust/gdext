use super::*;
use proc_macro2::Group;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(super) struct GodotConfigurationAny(pub(crate) GodotConfigurationPredicateList);

impl TryFrom<Group> for GodotConfigurationAny {
    type Error = GodotConditionCompilationError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        Ok(Self(GodotConfigurationPredicateList::try_from(group)?))
    }
}

impl GodotConditionalCompilation for GodotConfigurationAny {
    fn should_compile(&self) -> bool {
        self.0.iter().any(|option| option.should_compile())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::godot_cfg::option::GodotConfigurationOption;
    use crate::godot_cfg::GodotConfigurationPredicate;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

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
                    GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
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
                    GodotConfigurationOption::DocTest
                ),])
            )))
        );
    }
}
