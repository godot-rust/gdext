use godot::obj::{bounds, Bounds, DynGd, DynGdMut};
use godot::prelude::*;
use std::marker::PhantomData;

trait Health {}

#[derive(GodotClass)]
#[class(init)]
struct Thing {}

fn guard<'a, T>(gd: &'a mut Gd<T>, _type: TypeCapsule<T>) -> GdMut<'a, T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    gd.bind_mut()
}

#[derive(Copy, Clone)]
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


fn test() {
    let user_obj = Thing {};
    // let d = dyn_gd!(Health; node);

    let type_ = TypeCapsule::new(&user_obj);
    let _dyn_gd = {
        let gd = Gd::from_object(user_obj);

        let type_ = type_.clone();
        let downcast =move |obj: &Gd<Object>| {
            let mut concrete: Gd<_> = obj.clone().cast();
            // if false {
            //     std::mem::swap(&mut user_obj, &mut concrete);
            // }

            let guard = guard(&mut concrete, type_);
            DynGdMut::from_guard_type_inference(guard, |t| t)
        };

        DynGd::new(gd, downcast)
    };
}
