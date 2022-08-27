use crate::GodotClass;

pub struct ClassBuilder<C> {}
impl<C> ClassBuilder<C>
where
    C: GodotClass,
{
    // pub fn virtual_method(&self) -> MethodBuilder {
    //
    // }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct MethodBuilder<'cb, C, F> {}

impl<'cb, C, F> MethodBuilder<'cb, C, F> {
    pub(super) fn new(class_builder: &'cb ClassBuilder<C>, name: &'cb str, method: F) -> Self {}
}
