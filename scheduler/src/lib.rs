pub mod streamer;
pub use chrono::DateTime;
pub use chrono::Duration;
use libloading::Library;

#[allow(improper_ctypes_definitions)]
pub type PluginFrame = unsafe extern "C" fn(cairo::Context, f64, f64, Duration) -> bool;
pub struct Plugin<'a> {
    pub library: Library,
    pub frame: libloading::Symbol<'a, PluginFrame>,
}

impl Plugin<'_> {
    pub fn load(path: &str) -> Self {
        unsafe {
            let library = Library::new(std::path::Path::new(path).canonicalize().unwrap()).unwrap();
            let load = library.get::<unsafe extern "C" fn()>(b"load").unwrap();
            let frame = (*(&library as *const Library))
                .get::<PluginFrame>(b"frame")
                .unwrap();
            load();
            Self { library, frame }
        }
    }
}
