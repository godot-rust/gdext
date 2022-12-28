use super::*;
use proc_macro2::{Ident, TokenTree};

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
            // _ => todo!("(Unreachable) Work out how to evaluate cfg with an arbitrary string)"),
        }
    }
}

impl TryFrom<&Ident> for GodotConfigurationOption {
    type Error = GodotConditionCompilationError;

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
            _ => Err(Self::Error::OptionUnsupported(ident.clone())),
        }
    }
}

impl TryFrom<TokenTree> for GodotConfigurationOption {
    type Error = GodotConditionCompilationError;

    fn try_from(tt: TokenTree) -> Result<Self, Self::Error> {
        if let TokenTree::Ident(ident) = &tt {
            ident.try_into()
        } else {
            Err(Self::Error::OptionInvalidToken(tt))
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
        assert_eq!(option, GodotConfigurationOption::Test);

        let ts = TokenStream::from_str("doctest").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let option: GodotConfigurationOption = tt.try_into().unwrap();
        assert!(!option.should_compile());
        assert_eq!(option, GodotConfigurationOption::DocTest);
    }

    #[test]
    fn test_option_unsupported() {
        let ts = TokenStream::from_str("not_a_real_option").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConditionCompilationError::OptionUnsupported(_)) => {}
            _ => panic!("incorrect error returned {:?}", result),
        }
    }

    #[test]
    fn test_invalid_token() {
        let ts = TokenStream::from_str("(doctest)").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConditionCompilationError::OptionInvalidToken(_)) => {}
            _ => panic!("incorrect error returned {:?}", result),
        }

        let ts = TokenStream::from_str(";").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConditionCompilationError::OptionInvalidToken(_)) => {}
            _ => panic!("incorrect error returned {:?}", result),
        }

        let ts = TokenStream::from_str("123").unwrap();
        let tt = ts.into_iter().next().unwrap();
        let result = GodotConfigurationOption::try_from(tt);
        match result {
            Err(GodotConditionCompilationError::OptionInvalidToken(_)) => {}
            _ => panic!("incorrect error returned {:?}", result),
        }
    }
}
