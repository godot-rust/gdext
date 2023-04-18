/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

macro_rules! impl_rust_string_conv {
    ($Ty:ty) => {
        impl<S> From<S> for $Ty
        where
            S: AsRef<str>,
        {
            fn from(string: S) -> Self {
                let intermediate = GodotString::from(string.as_ref());
                Self::from(&intermediate)
            }
        }

        impl From<&$Ty> for String {
            fn from(string: &$Ty) -> Self {
                let intermediate = GodotString::from(string);
                Self::from(&intermediate)
            }
        }

        impl From<$Ty> for String {
            fn from(string: $Ty) -> Self {
                Self::from(&string)
            }
        }

        impl std::str::FromStr for $Ty {
            type Err = std::convert::Infallible;

            fn from_str(string: &str) -> Result<Self, Self::Err> {
                Ok(Self::from(string))
            }
        }
    };
}
