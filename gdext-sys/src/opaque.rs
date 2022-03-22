use std::marker::PhantomData;

// Stores an opaque object of a certain size, with very restricted operations
#[repr(C, align(8))]
pub struct Opaque<const N: usize> {
    storage: [u8; N],
    marker: PhantomData<*const u8>, // disable Send/Sync
}

impl<const N: usize> Opaque<N> {
    pub unsafe fn from_sys(pointer: *mut std::ffi::c_void) -> Self {
        Self {
            storage: *(pointer as *mut [u8; N]), // uses Copy
            marker: PhantomData,
        }
    }

    pub fn to_sys_mut(&mut self) -> *mut std::ffi::c_void {
        &mut self.storage as *mut [u8; N] as *mut std::ffi::c_void
    }

    // Note: returns mut pointer -- GDExtension API is not really const-correct
    // However, this doesn't borrow the enclosing object exclusively, i.e. caller
    // has some influence (however type system can only help in limited ways)
    pub fn to_sys(&self) -> *mut std::ffi::c_void {
        &self.storage as *const [u8; N] as *mut std::ffi::c_void
    }
}
