/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//use crate as sys;
use std::collections::HashSet;
use std::ffi::CString;

// Retains values indefinitely (effectively 'static).
//
// This is unfortunately necessary with the current GDExtension design.
// E.g. a callback from Godot will invoke Rust binding to get information about registered properties in a class.
// This callback returns a PropertyInfo struct, with several char* attributes like `name`, `class_name`, etc.
// Now, when returning this value into the engine, the backing memory must remain alive. And since we don't know
// how long the engine uses them, this might take until termination of the binding (i.e. 'static).
//
// This is at least the conservative approach, but wastes quite some memory. At least the things are lazily loaded.
// An alternative would be to retain only one value at a time for a certain "domain" (e.g. "property list"). This
// would require knowledge and reliance on Godot's implementation. In other words, strings would only be retained until
// the next callback is invoked, which can overwrite the string value. If the same memory locations would be reused
// by accident, Godot would mostly display the wrong strings (logic error instead of UB). With significantly large
// buffers (CString::reserve()), UB could be avoided entirely.
//
// Reported at https://github.com/godotengine/godot/issues/61968
#[derive(Default)]
pub struct GlobalRegistry {
    c_strings: HashSet<CString>,
}

impl GlobalRegistry {
    pub fn c_string(&mut self, s: &str) -> *const i8 {
        let value = CString::new(s).unwrap_or_else(|_| panic!("Invalid string '{s}'"));

        if let Some(existing) = self.c_strings.get(&value) {
            //println!("<<< Cache '{s}'");
            existing.as_ptr()
        } else {
            //println!(">>> Store '{s}'     [total={}]", self.c_strings.len()+1);
            let copy = value.clone();
            self.c_strings.insert(value);
            let new = self.c_strings.get(&copy).unwrap();
            new.as_ptr()
        }
    }

    // fn property_info<T>(&mut self, property_name: &str) -> sys::GDExtensionPropertyInfo {
    //
    // }
}
