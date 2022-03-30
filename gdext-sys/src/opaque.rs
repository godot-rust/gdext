use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

// Note: transmute not supported for const generics; see
// https://users.rust-lang.org/t/transmute-in-the-context-of-constant-generics/56827

// Stores an opaque object of a certain size, with very restricted operations
#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct Opaque<const N: usize> {
    storage: [u8; N],
    marker: PhantomData<*const u8>, // disable Send/Sync
}

impl<const N: usize> Opaque<N> {
    pub unsafe fn with_init(init: impl FnOnce(*mut c_void)) -> Self {
        let mut raw = MaybeUninit::<[u8; N]>::uninit();
        init(raw.as_mut_ptr() as *mut c_void);

        Self {
            storage: raw.assume_init(),
            marker: PhantomData,
        }
    }

    pub unsafe fn with_value_init(init: impl FnOnce(*mut c_void)) -> Self {
        let mut raw = MaybeUninit::<[u8; N]>::uninit();
        init((raw.as_mut_ptr() as *mut *mut c_void).read());

        Self {
            storage: raw.assume_init(),
            marker: PhantomData,
        }
    }

    pub unsafe fn from_sys(pointer: *mut c_void) -> Self {
        Self {
            storage: (&pointer as *const _ as *mut [u8; N]).read(), // uses Copy
            marker: PhantomData,
        }
    }

    pub unsafe fn from_value_sys(pointer: *mut c_void) -> Self {
        Self {
            // storage: transmute(pointer),
            storage: (pointer as *mut [u8; N]).read(), // uses Copy
            marker: PhantomData,
        }
    }

    pub fn to_sys_mut(&mut self) -> *mut c_void {
        self.to_sys()
    }

    // Note: returns mut pointer -- GDExtension API is not really const-correct
    // However, this doesn't borrow the enclosing object exclusively, i.e. caller
    // has some influence (however type system can only help in limited ways)
    pub fn to_sys(&self) -> *mut c_void {
        &self.storage as *const [u8; N] as *mut c_void
    }

    pub fn to_value_sys(&self) -> *mut c_void {
        unsafe { (&self.storage as *const [u8; N] as *mut *mut c_void).read() }
        // unsafe { transmute(self.storage) }
    }
}

impl<const N: usize> std::fmt::Display for Opaque<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "^{:?}", self.storage)
    }
}
