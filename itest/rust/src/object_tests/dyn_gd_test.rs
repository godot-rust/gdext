use godot::obj::{bounds, Bounds, DynGdMut, DynGd};
use godot::prelude::*;

trait Health {}

#[derive(GodotClass)]
#[class(init)]
struct Thing {}

fn guard<'a, T>(gd: &'a mut Gd<T>, user_obj_inference: &T) -> GdMut<'a, T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    gd.bind_mut()
}

fn test() {
    let user_obj = Thing {};
    // let d = dyn_gd!(Health; node);

    {
        let gd = Gd::from_object(user_obj);

        let downcast = |obj: Gd<Object>| {
            let mut concrete: Gd<_> = obj.cast();
            // if false {
            //     std::mem::swap(&mut user_obj, &mut concrete);
            // }

            let guard = guard(&mut concrete, &user_obj);
            DynGdMut::from_guard_type_inference(guard, |t| t, &user_obj)
        };

        DynGd::new(gd, downcast)
    }
}
