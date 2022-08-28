use crate::GodotClass;
use std::marker::PhantomData;

mod method;

pub struct ClassBuilder<C> {
    _c: PhantomData<C>,
}

impl<C> ClassBuilder<C>
where
    C: GodotClass,
{
    pub fn virtual_method<'cb, F>(&'cb mut self, name: &'cb str, method: F) -> MethodBuilder<C, F> {
        MethodBuilder::new(self, name, method)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)] // TODO rm
pub struct MethodBuilder<'cb, C, F> {
    class_builder: &'cb mut ClassBuilder<C>,
    name: &'cb str,
    method: F,
}

impl<'cb, C, F> MethodBuilder<'cb, C, F> {
    pub(super) fn new(class_builder: &'cb mut ClassBuilder<C>, name: &'cb str, method: F) -> Self {

        Self {
            class_builder,
            name,
            method
        }
    }
}
