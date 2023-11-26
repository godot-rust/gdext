use std::sync;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::obj::{Base, GodotClass};
use crate::out;

use super::Lifecycle;

pub struct AtomicLifecycle {
    atomic: AtomicU32,
}

impl AtomicLifecycle {
    pub fn new(value: Lifecycle) -> Self {
        Self {
            atomic: AtomicU32::new(value as u32),
        }
    }

    pub fn get(&self) -> Lifecycle {
        match self.atomic.load(Ordering::Relaxed) {
            0 => Lifecycle::Alive,
            1 => Lifecycle::Destroying,
            other => panic!("invalid lifecycle {other}"),
        }
    }

    pub fn set(&self, lifecycle: Lifecycle) {
        let value = match lifecycle {
            Lifecycle::Alive => 0,
            Lifecycle::Destroying => 1,
        };

        self.atomic.store(value, Ordering::Relaxed);
    }
}

/// Manages storage and lifecycle of user's extension class instances.
pub struct InstanceStorage<T: GodotClass> {
    user_instance: sync::RwLock<T>,
    pub(super) base: Base<T::Base>,

    // Declared after `user_instance`, is dropped last
    pub(super) lifecycle: AtomicLifecycle,
    godot_ref_count: AtomicU32,
}

/// For all Godot extension classes
impl<T: GodotClass> InstanceStorage<T> {
    /// Returns a write guard (even if poisoned), or `None` when the lock is held by another thread.
    /// This might need adjustment if threads should await locks.
    #[must_use]
    fn write_ignoring_poison(&self) -> Option<sync::RwLockWriteGuard<T>> {
        match self.user_instance.try_write() {
            Ok(guard) => Some(guard),
            Err(sync::TryLockError::Poisoned(poison_error)) => Some(poison_error.into_inner()),
            Err(sync::TryLockError::WouldBlock) => None,
        }
    }

    /// Returns a read guard (even if poisoned), or `None` when the lock is held by another writing thread.
    /// This might need adjustment if threads should await locks.
    #[must_use]
    fn read_ignoring_poison(&self) -> Option<sync::RwLockReadGuard<T>> {
        match self.user_instance.try_read() {
            Ok(guard) => Some(guard),
            Err(sync::TryLockError::Poisoned(poison_error)) => Some(poison_error.into_inner()),
            Err(sync::TryLockError::WouldBlock) => None,
        }
    }

    // fn __static_type_check() {
    //     enforce_sync::<InstanceStorage<T>>();
    // }
}

impl<T: GodotClass> super::Storage for InstanceStorage<T> {
    type Instance = T;

    type RefGuard<'a> = sync::RwLockReadGuard<'a, T>;

    type MutGuard<'a> = sync::RwLockWriteGuard<'a, T>;

    fn construct(
        user_instance: Self::Instance,
        base: Base<<Self::Instance as GodotClass>::Base>,
    ) -> Self {
        out!("    Storage::construct             <{:?}>", base);

        Self {
            user_instance: sync::RwLock::new(user_instance),
            base,
            lifecycle: AtomicLifecycle::new(Lifecycle::Alive),
            godot_ref_count: AtomicU32::new(1),
        }
    }

    fn is_bound(&self) -> bool {
        // Needs to borrow mutably, otherwise it succeeds if shared borrows are alive.
        self.write_ignoring_poison().is_none()
    }

    fn base(&self) -> &Base<<Self::Instance as GodotClass>::Base> {
        &self.base
    }

    fn get(&self) -> Self::RefGuard<'_> {
        self.read_ignoring_poison().unwrap_or_else(|| {
            panic!(
                "Gd<T>::bind() failed, already bound; obj = {}.\n  \
                     Make sure there is no &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                self.base,
            )
        })
    }

    fn get_mut(&self) -> Self::MutGuard<'_> {
        self.write_ignoring_poison().unwrap_or_else(|| {
            panic!(
                "Gd<T>::bind_mut() failed, already bound; obj = {}.\n  \
                     Make sure there is no &T or &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                self.base,
            )
        })
    }

    fn get_lifecycle(&self) -> Lifecycle {
        self.lifecycle.get()
    }

    fn set_lifecycle(&self, lifecycle: Lifecycle) {
        self.lifecycle.set(lifecycle)
    }
}

impl<T: GodotClass> super::StorageRefCounted for InstanceStorage<T> {
    fn godot_ref_count(&self) -> u32 {
        self.godot_ref_count.load(Ordering::Relaxed)
    }

    fn on_inc_ref(&self) {
        self.godot_ref_count.fetch_add(1, Ordering::Relaxed);
        out!(
            "    Storage::on_inc_ref (rc={})     <{:?}>",
            self.godot_ref_count(),
            self.base,
        );
    }

    fn on_dec_ref(&self) {
        self.godot_ref_count.fetch_sub(1, Ordering::Relaxed);
        out!(
            "  | Storage::on_dec_ref (rc={})     <{:?}>",
            self.godot_ref_count(),
            self.base,
        );
    }
}

// TODO make InstanceStorage<T> Sync
// This type can be accessed concurrently from multiple threads, so it should be Sync. That implies however that T must be Sync too
// (and possibly Send, because with `&mut` access, a `T` can be extracted as a value using mem::take() etc.).
// Which again means that we need to infest half the codebase with T: Sync + Send bounds, *and* make it all conditional on
// `#[cfg(feature = "experimental-threads")]`.
//
// A better design would be a distinct Gds<T: Sync> pointer, which requires synchronized.
// This needs more design on the multi-threading front (#18).
//
// The following code + __static_type_check() above would make sure that InstanceStorage is Sync.
// Make sure storage is Sync in multi-threaded case, as it can be concurrently accessed through aliased Gd<T> pointers.
// fn enforce_sync<T: Sync>() {}
