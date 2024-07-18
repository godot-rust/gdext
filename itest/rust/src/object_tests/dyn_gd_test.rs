use godot::obj::{bounds, Bounds, DynGd, DynGdMut};
use godot::prelude::*;
use std::marker::PhantomData;

trait Health {}

#[derive(GodotClass)]
#[class(init)]
struct Thing {}

impl Health for Thing {}

fn guard<'a, T>(gd: &'a mut Gd<T>, _type: TypeCapsule<T>) -> GdMut<'a, T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    gd.bind_mut()
}

/// ZST with the only purpose to carry type information.
struct TypeCapsule<T> {
    _type: PhantomData<*const T>,
}
impl<T> TypeCapsule<T> {
    pub fn new(_ref: &T) -> Self {
        Self {
            _type: PhantomData::<*const T>,
        }
    }
}
impl<T> Clone for TypeCapsule<T> {
    fn clone(&self) -> Self {
        Self { _type: PhantomData }
    }
}
impl<T> Copy for TypeCapsule<T> {}

fn test() {
    let user_obj = Thing {};
    // let d = dyn_gd!(Health; node);

    let type_ = TypeCapsule::new(&user_obj);
    let mut dyn_gd = {
        let gd = Gd::from_object(user_obj);

        // let type_ = type_.clone();
        let downcast = move |obj: &mut Gd<Object>| {
            // let mut concrete: Gd<_> = obj.clone().cast();
            let concrete: &mut Gd<_> = unsafe { std::mem::transmute(obj) };

            // if false {
            //     std::mem::swap(&mut user_obj, &mut concrete);
            // }

            let guard = guard(concrete, type_);
            DynGdMut::from_guard(guard, |t| -> &mut dyn Health { t })
        };

        DynGd::<_, dyn Health>::new(gd, downcast)
    };

    let t = dyn_gd.dbind_mut();
    let _ = t;
}
