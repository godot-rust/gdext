use gdext_sys as sys;
use std::collections::btree_map::BTreeMap;

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub static mut INIT_OPTIONS: Option<InitHandle> = None;

struct LoadResult {
    success: bool,
}

pub trait ExtensionLib {
    fn load_library(mut handle: InitHandle) -> bool {
        handle.register_layer(InitLevel::Scene, default_layer);

        true
    }
}

pub trait ExtensionLayer {
    type UserData;

    fn initialize(level: InitLevel, user_data: &mut Self::UserData);
    fn deinitialize(level: InitLevel, user_data: &mut Self::UserData);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct DefaultLayer;

impl ExtensionLayer for DefaultLayer {
    type UserData = ();

    fn initialize(level: InitLevel, user_data: &mut Self::UserData) {
        todo!()
    }

    fn deinitialize(level: InitLevel, user_data: &mut Self::UserData) {
        todo!()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct InitHandle {
    layers: BTreeMap<InitLevel, Box<dyn ExtensionLayer>>,
}

impl InitHandle {
    pub fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
        }
    }

    pub fn register_layer(&mut self, level: InitLevel, layer: impl ExtensionLayer<UserData = ()>) {
        self.layers.insert(level, Box::new(layer));
    }

    pub fn register_layer_with<T>(
        &mut self,
        level: InitLevel,
        layer: impl ExtensionLayer<UserData = T>,
        user_data: T,
    ) {
        self.layers.insert(level, Box::new(layer));
    }

    pub fn lowest_init_level(&self) -> InitLevel {
        self.layers
            .first_key_value()
            .map(|(k, _v)| *k)
            .unwrap_or(InitLevel::Scene)
    }

    pub fn run_init_function(&mut self, level: InitLevel) {
        if let Some(f) = self.init_levels.remove(&level) {
            f();
        }
    }

    pub fn run_deinit_function(&mut self, level: InitLevel) {
        if let Some(f) = self.deinit_levels.remove(&level) {
            f();
        }
    }
}

impl Default for InitHandle {
    fn default() -> Self {
        Self::new()
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum InitLevel {
    Core,
    Servers,
    Scene,
    Editor,
}

impl InitLevel {
    #[doc(hidden)]
    pub fn from_sys(level: gdext_sys::GDNativeInitializationLevel) -> Self {
        match level {
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_CORE => Self::Core,
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SERVERS => Self::Servers,
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE => Self::Scene,
            sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_EDITOR => Self::Editor,
            _ => {
                println!("WARNING: unknown initialization level {}", level);
                Self::Scene
            }
        }
    }
    #[doc(hidden)]
    pub fn to_sys(self) -> gdext_sys::GDNativeInitializationLevel {
        match self {
            Self::Core => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_CORE,
            Self::Servers => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SERVERS,
            Self::Scene => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_SCENE,
            Self::Editor => sys::GDNativeInitializationLevel_GDNATIVE_INITIALIZATION_EDITOR,
        }
    }
}
