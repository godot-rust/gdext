#![macro_use]

#[macro_export]
macro_rules! gdext_print_warning {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            gdext_sys::interface_fn!(print_warning)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

#[macro_export]
macro_rules! gdext_print_error {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            gdext_sys::interface_fn!(print_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

#[macro_export]
macro_rules! gdext_print_script_error {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            gdext_sys::interface_fn!(print_script_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}
