use godot::obj::DynGdMut;
use godot::prelude::*;

trait Health {}

#[derive(GodotClass)]
#[class(init)]
struct Thing {

}

fn test() {
	let user_obj = Thing {};
	// let d = dyn_gd!(Health; node);

	{
		let gd = Gd::from_object(user_obj);

		let downcast = |obj: Gd<Object>| {
			let concrete: Gd<_> = obj.cast();
			if false {
				std::mem::swap(&mut user_obj, &mut concrete);
			}
			DynGdMut::from_guard(concrete.bind_mut())
		};

		make_dyn(downcast)
	}
}