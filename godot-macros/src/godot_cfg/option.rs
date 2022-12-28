use std::error::Error;
use crate::godot_cfg::GodotConditionalCompilation;
use proc_macro2::{Ident, TokenTree};
use std::fmt::{Display, Formatter};

/// Possible errors when trying to pass a cfg option
#[derive(Debug, Clone)]
pub enum GodotConfigurationOptionError {
    UnsupportedOption(Ident),
    // OptionRequiresValue(Ident),
    // OptionDoesNotSupportValue(Ident),
    InvalidToken(TokenTree),
}

impl Display for GodotConfigurationOptionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ve: venial::Error = self.clone().into();
        write!(f, "{}", ve)
    }
}

impl Error for GodotConfigurationOptionError {}

impl Into<venial::Error> for GodotConfigurationOptionError {
    fn into(self) -> venial::Error {
        match self {
            Self::UnsupportedOption(ident) => {
                let message = format!("Unsupported conditional compilation option: {}", &ident);
                venial::Error::new_at_tokens(ident, message)
            }
            // Self::OptionRequiresValue(ident) => {
            //     let message = format!("Conditional compilation option requires a value: {}", &ident);
            //     venial::Error::new_at_tokens(ident, message)
            // }
            // Self::OptionDoesNotSupportValue(ident) => {
            //     let message = format!("Conditional compilation option does not support a value: {}", &ident);
            //     venial::Error::new_at_tokens(ident, message)
            // }
            Self::InvalidToken(tt) => {
                let message = format!("Invalid conditional compilation option: {}", &tt);
                venial::Error::new_at_tokens(tt, message)
            }
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum GodotConfigurationOption {
    Test,
    DocTest,
    Unix,
    Windows,
    DebugAssertions,
    ProcMacro,
    // Feature(String),
    // TargetArch(String),
    // TargetFeature(String),
    // TargetOs(String),
    // TargetFamily(String),
    // TargetEnv(String),
    // TargetEndian(String),
    // TargetPointerWidth(String),
    // TargetVendor(String),
    // TargetHasAtomic(String),
    // Panic(String),
}

impl GodotConditionalCompilation for GodotConfigurationOption {
    fn should_compile(&self) -> bool {
        match self {
            Self::Test => cfg!(test),
            Self::DocTest => cfg!(doctest),
            Self::Unix => cfg!(unix),
            Self::Windows => cfg!(windows),
            Self::DebugAssertions => cfg!(debug_assertions),
            Self::ProcMacro => cfg!(proc_macro),
            // Self::Feature(s) => cfg!(feature = s),
            // Self::TargetArch(s) => cfg!(target_arch = s),
            // Self::TargetFeature(s) => cfg!(target_feature = s),
            // Self::TargetOs(s) => cfg!(target_os = s),
            // Self::TargetFamily(s) => cfg!(target_family = s),
            // Self::TargetEnv(s) => cfg!(target_env = s),
            // Self::TargetEndian(s) => cfg!(target_endian = s),
            // Self::TargetPointerWidth(s) => cfg!(target_pointer_width = s),
            // Self::TargetVendor(s) => cfg!(target_vendor = s),
            // Self::TargetHasAtomic(s) => cfg!(target_has_atomic = s),
            // Self::Panic(s) => cfg!(panic = s),
            _ => todo!("(Unreachable) Work out how to evaluate cfg with an arbitrary string)")
        }
    }
}

impl TryFrom<&Ident> for GodotConfigurationOption {
    type Error = GodotConfigurationOptionError;

    fn try_from(ident: &Ident) -> Result<Self, Self::Error> {
        let ident_string = ident.to_string();
        match ident_string.as_str() {
            "test" => Ok(Self::Test),
            "doctest" => Ok(Self::DocTest),
            "unix" => Ok(Self::Unix),
            "windows" => Ok(Self::Windows),
            "debug_assertions" => Ok(Self::DebugAssertions),
            "proc_macro" => Ok(Self::ProcMacro),
            // "feature"
            // | "target_arch"
            // | "target_feature"
            // | "target_os"
            // | "target_family"
            // | "target_env"
            // | "target_endian"
            // | "target_pointer_width"
            // | "target_vendor"
            // | "target_has_atomic"
            // | "panic" => Err(Self::Error::OptionRequiresValue(ident.clone())),
            _ => Err(Self::Error::UnsupportedOption(ident.clone())),
        }
    }
}

impl TryFrom<TokenTree> for GodotConfigurationOption {
    type Error = GodotConfigurationOptionError;

    fn try_from(tt: TokenTree) -> Result<Self, Self::Error> {
        if let TokenTree::Ident(ident) = &tt {
            ident.try_into()
        } else {
            Err(Self::Error::InvalidToken(tt))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use std::str::FromStr;

    #[test]
    fn test_option() {
        let ts = TokenStream::from_str("test").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let option: GodotConfigurationOption = tt.try_into().unwrap();
        assert!(option.should_compile());
        assert_eq!(
            option,
            GodotConfigurationOption::Test
        );

        let ts = TokenStream::from_str("doctest").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let option: GodotConfigurationOption = tt.try_into().unwrap();
        assert!(!option.should_compile());
        assert_eq!(
            option,
            GodotConfigurationOption::DocTest
        );
    }

    #[test]
    fn test_unsupported_option() {
        let ts = TokenStream::from_str("not_a_real_option").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConfigurationOptionError::UnsupportedOption(_)) => {},
            _ => panic!("incorrect error returned {:?}", result),
        }
    }

    #[test]
    fn test_invalid_token() {
        let ts = TokenStream::from_str("(doctest)").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConfigurationOptionError::InvalidToken(_)) => {},
            _ => panic!("incorrect error returned {:?}", result),
        }

        let ts = TokenStream::from_str(";").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConfigurationOptionError::InvalidToken(_)) => {},
            _ => panic!("incorrect error returned {:?}", result),
        }

        let ts = TokenStream::from_str("123").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConfigurationOptionError::InvalidToken(_)) => {},
            _ => panic!("incorrect error returned {:?}", result),
        }
    }
}
