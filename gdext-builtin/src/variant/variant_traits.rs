use crate::Variant;

pub trait FromVariant: Sized {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError>;

    fn from_variant(variant: &Variant) -> Self {
        Self::try_from_variant(variant).unwrap_or_else(|e| {
            panic!(
                "failed to convert from variant {:?} to {}; {:?}",
                std::any::type_name::<Self>(),
                variant,
                e
            )
        })
    }
}

pub trait ToVariant {
    /*fn try_to_variant(&self) -> Result<Variant, VariantConversionError>;

    fn to_variant(&self) -> Variant {
        Self::try_to_variant(self).unwrap_or_else(|e| {
            panic!(
                "failed to convert from {} to variant; {:?}",
                std::any::type_name::<Self>(),
                e
            )
        })
    }*/

    fn to_variant(&self) -> Variant;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct VariantConversionError;
