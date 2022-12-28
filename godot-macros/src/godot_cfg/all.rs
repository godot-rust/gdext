use crate::godot_cfg::list::{
    GodotConfigurationPredicateList, GodotConfigurationPredicateListError,
};
use crate::godot_cfg::GodotConditionalCompilation;
use proc_macro2::Group;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct GodotConfigurationAll(pub(crate) GodotConfigurationPredicateList);

impl TryFrom<Group> for GodotConfigurationAll {
    type Error = GodotConfigurationPredicateListError;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        Ok(Self(GodotConfigurationPredicateList::try_from(group)?))
    }
}

impl GodotConditionalCompilation for GodotConfigurationAll {
    fn should_compile(&self) -> bool {
        self.0.iter().all(|predicate| predicate.should_compile())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::godot_cfg::{GodotConfigurationOption, GodotConfigurationPredicate};
    use proc_macro2::{TokenStream, TokenTree};
    use std::str::FromStr;

    #[test]
    fn test_all() {
        let tt = TokenStream::from_str("(test, test)")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let predicate = if let TokenTree::Group(group) = tt {
            GodotConfigurationAll::try_from(group).unwrap()
        } else {
            panic!("token returned was not a group")
        };
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationAll(GodotConfigurationPredicateList(vec![
                GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
            ]))
        );

        let tt = TokenStream::from_str("(test, doctest)")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let predicate = if let TokenTree::Group(group) = tt {
            GodotConfigurationAll::try_from(group).unwrap()
        } else {
            panic!("token returned was not a group")
        };
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationAll(GodotConfigurationPredicateList(vec![
                GodotConfigurationPredicate::Option(GodotConfigurationOption::Test),
                GodotConfigurationPredicate::Option(GodotConfigurationOption::DocTest),
            ]))
        );
    }
}
