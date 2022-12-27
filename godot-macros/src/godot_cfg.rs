use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug)]
enum GodotConditionalCompilationError {
    UnsupportedOption(String),
    EmptyIdentifier,
    UnableToParse(String),
}

impl Display for GodotConditionalCompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GodotConditionalCompilationError::UnsupportedOption(e) => {
                write!(f, "Unsupported Configuration Option '{}'", e)
            }
            GodotConditionalCompilationError::EmptyIdentifier => {
                write!(f, "Empty predicate found in godot_cfg")
            }
            GodotConditionalCompilationError::UnableToParse(e) => {
                write!(f, "Unable to parse '{}'", e)
            }
        }
    }
}

impl Error for GodotConditionalCompilationError {}

trait GodotConditionalCompilation {
    fn should_compile(&self) -> bool;
}

// ConfigurationPredicate doesn't seem to be accessible so we'll make our own.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum GodotConfigurationPredicate {
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

impl FromStr for GodotConfigurationPredicate {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        match &trimmed[..3] {
            "all" => Ok(Self::All(Box::new(GodotConfigurationAll::from_str(s)?))),
            "any" => Ok(Self::Any(Box::new(GodotConfigurationAny::from_str(s)?))),
            "not" => Ok(Self::Not(Box::new(GodotConfigurationNot::from_str(s)?))),
            _ => Ok(Self::Option(GodotConfigurationOption::from_str(s)?)),
        }
    }
}

// Currently only Test and DocTest are supported. I'm not sure the other things supported by cfg!
// necessarily make sense to toggle the rest of the godot on or off but they can be added here later
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum GodotConfigurationIdentifier {
    Test,
    DocTest,
}

impl FromStr for GodotConfigurationIdentifier {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // We could lowercase this, while cfg! is type sensitive, we're not bound by that
        let trimmed = s.trim();
        match trimmed {
            "test" => Ok(Self::Test),
            "doctest" => Ok(Self::DocTest),
            _ => Err(Self::Err::UnsupportedOption(trimmed.to_string())),
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct GodotConfigurationOption {
    identifier: GodotConfigurationIdentifier,
    value: Option<String>,
}

impl FromStr for GodotConfigurationOption {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let mut split = trimmed.split('=');
        let identifier = GodotConfigurationIdentifier::from_str(
            split
                .next()
                .ok_or(GodotConditionalCompilationError::EmptyIdentifier)?,
        )?;
        match identifier {
            GodotConfigurationIdentifier::Test => Ok(Self {
                identifier,
                value: None,
            }),
            GodotConfigurationIdentifier::DocTest => Ok(Self {
                identifier,
                value: None,
            }),
        }
    }
}

impl GodotConditionalCompilation for GodotConfigurationOption {
    fn should_compile(&self) -> bool {
        match self.identifier {
            GodotConfigurationIdentifier::Test => cfg!(test),
            GodotConfigurationIdentifier::DocTest => cfg!(doctest),
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct GodotConfigurationAll(GodotConfigurationPredicateList);

impl FromStr for GodotConfigurationAll {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.starts_with("all(") && trimmed.ends_with(')') {
            let inner = &trimmed[4..trimmed.len() - 1];
            Ok(GodotConfigurationAll(
                GodotConfigurationPredicateList::from_str(inner)?,
            ))
        } else {
            Err(Self::Err::UnableToParse(s.to_string()))
        }
    }
}

impl GodotConditionalCompilation for GodotConfigurationAll {
    fn should_compile(&self) -> bool {
        self.0.iter().all(|option| option.should_compile())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct GodotConfigurationAny(GodotConfigurationPredicateList);

impl FromStr for GodotConfigurationAny {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.starts_with("any(") && trimmed.ends_with(')') {
            let inner = &trimmed[4..trimmed.len() - 1];
            Ok(GodotConfigurationAny(
                GodotConfigurationPredicateList::from_str(inner)?,
            ))
        } else {
            Err(Self::Err::UnableToParse(s.to_string()))
        }
    }
}

impl GodotConditionalCompilation for GodotConfigurationAny {
    fn should_compile(&self) -> bool {
        self.0.iter().any(|option| option.should_compile())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct GodotConfigurationNot(GodotConfigurationPredicate);

impl FromStr for GodotConfigurationNot {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.starts_with("not(") && trimmed.ends_with(')') {
            let inner = &trimmed[4..trimmed.len() - 1];
            Ok(Self(GodotConfigurationPredicate::from_str(inner)?))
        } else {
            Err(Self::Err::UnableToParse(s.to_string()))
        }
    }
}

impl GodotConditionalCompilation for GodotConfigurationNot {
    fn should_compile(&self) -> bool {
        !self.0.should_compile()
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct GodotConfigurationPredicateList(Vec<GodotConfigurationPredicate>);

impl Deref for GodotConfigurationPredicateList {
    type Target = Vec<GodotConfigurationPredicate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for GodotConfigurationPredicateList {
    type Err = GodotConditionalCompilationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Todo: This is not an acceptable way to parse the list as it may contain nested lists
        let predicates: Result<Vec<_>, _> = s
            .split(',')
            .map(GodotConfigurationPredicate::from_str)
            .collect();
        Ok(Self(predicates?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option() {
        let s = "test";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Option(GodotConfigurationOption {
                identifier: GodotConfigurationIdentifier::Test,
                value: None,
            })
        );

        let s = "doctest";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Option(GodotConfigurationOption {
                identifier: GodotConfigurationIdentifier::DocTest,
                value: None,
            })
        );
    }

    #[test]
    fn test_all() {
        let s = "all(test, test)";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::All(Box::new(GodotConfigurationAll(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::Test,
                        value: None,
                    }),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::Test,
                        value: None,
                    }),
                ])
            )))
        );

        let s = "all(test, doctest)";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::All(Box::new(GodotConfigurationAll(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::Test,
                        value: None,
                    }),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::DocTest,
                        value: None,
                    }),
                ])
            )))
        );
    }

    #[test]
    fn test_any() {
        let s = "any(test, doctest)";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Any(Box::new(GodotConfigurationAny(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::Test,
                        value: None,
                    }),
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::DocTest,
                        value: None,
                    }),
                ])
            )))
        );

        let s = "any(doctest)";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Any(Box::new(GodotConfigurationAny(
                GodotConfigurationPredicateList(vec![
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::DocTest,
                        value: None,
                    }),
                ])
            )))
        );
    }

    #[test]
    fn test_not() {
        let s = "not(test)";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(!predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                GodotConfigurationPredicate::Option(GodotConfigurationOption {
                    identifier: GodotConfigurationIdentifier::Test,
                    value: None,
                })
            )))
        );

        let s = "not(not(test))";
        let predicate = GodotConfigurationPredicate::from_str(s).unwrap();
        assert!(predicate.should_compile());
        assert_eq!(
            predicate,
            GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
                    GodotConfigurationPredicate::Option(GodotConfigurationOption {
                        identifier: GodotConfigurationIdentifier::Test,
                        value: None,
                    })
                )))
            )))
        );
    }

    // ToDo: Fix list parsing so this works
    // #[test]
    // fn test_everything() {
    //     let s = "any(all(test, not(doctest)), doctest)";
    //     let cond = GodotConfigurationPredicate::from_str(s).unwrap();
    //     assert!(cond.should_compile());
    //     assert_eq!(
    //         cond,
    //         GodotConfigurationPredicate::Any(Box::new(GodotConfigurationAny(
    //             GodotConfigurationPredicateList(vec![
    //                 GodotConfigurationPredicate::All(Box::new(GodotConfigurationAll(
    //                     GodotConfigurationPredicateList(vec![
    //                         GodotConfigurationPredicate::Option(GodotConfigurationOption {
    //                             identifier: GodotConfigurationIdentifier::Test,
    //                             value: None,
    //                         }),
    //                         GodotConfigurationPredicate::Not(Box::new(GodotConfigurationNot(
    //                             GodotConfigurationPredicate::Option(GodotConfigurationOption {
    //                                 identifier: GodotConfigurationIdentifier::DocTest,
    //                                 value: None,
    //                             })
    //                         ))),
    //                     ])
    //                 ))),
    //                 GodotConfigurationPredicate::Option(GodotConfigurationOption {
    //                     identifier: GodotConfigurationIdentifier::DocTest,
    //                     value: None,
    //                 }),
    //             ])
    //         )))
    //     )
    // }
}
