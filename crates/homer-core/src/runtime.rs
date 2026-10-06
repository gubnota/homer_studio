#[cfg(not(feature = "desktop"))]
pub use headless::*;
#[cfg(feature = "desktop")]
pub use tauri::{AppHandle, Manager, State, async_runtime, ipc, path};
#[cfg(not(feature = "desktop"))]
mod headless {
    use std::{ops::Deref, path::PathBuf, sync::Arc};
    #[derive(Clone)]
    pub struct AppHandle {
        pub data: PathBuf,
        pub resources: PathBuf,
        pub shared: Arc<crate::AppState>,
    }
    impl AppHandle {
        pub fn new(data: PathBuf, resources: PathBuf) -> Self {
            Self {
                data,
                resources,
                shared: Arc::new(crate::AppState::default()),
            }
        }
        pub fn path(&self) -> &Self {
            self
        }
        pub fn app_data_dir(&self) -> std::io::Result<PathBuf> {
            Ok(self.data.clone())
        }
        pub fn app_config_dir(&self) -> std::io::Result<PathBuf> {
            Ok(self.data.join("config"))
        }
        pub fn app_cache_dir(&self) -> std::io::Result<PathBuf> {
            Ok(self.data.join("cache"))
        }
        pub fn resource_dir(&self) -> std::io::Result<PathBuf> {
            Ok(self.resources.clone())
        }
        pub fn document_dir(&self) -> std::io::Result<PathBuf> {
            let p = self.data.join("projects");
            std::fs::create_dir_all(&p)?;
            Ok(p)
        }
        pub fn resolve(
            &self,
            p: impl AsRef<std::path::Path>,
            _: path::BaseDirectory,
        ) -> std::io::Result<PathBuf> {
            Ok(self.resources.join(p))
        }
        pub fn state<T: 'static>(&self) -> State<'_, T> {
            let any: &dyn std::any::Any = self.shared.as_ref();
            State(
                any.downcast_ref::<T>()
                    .expect("registered application state"),
            )
        }
    }
    pub trait Manager {}
    impl Manager for AppHandle {}
    pub struct State<'a, T>(&'a T);
    impl<'a, T> State<'a, T> {
        pub fn inner(&self) -> &'a T {
            self.0
        }
    }
    impl<T> Deref for State<'_, T> {
        type Target = T;
        fn deref(&self) -> &T {
            self.0
        }
    }
    pub mod path {
        pub enum BaseDirectory {
            Resource,
        }
    }
    pub mod async_runtime {
        pub use tokio::task::spawn_blocking;
    }
    pub mod ipc {
        pub struct Response(pub Vec<u8>);
        impl Response {
            pub fn new(bytes: Vec<u8>) -> Self {
                Self(bytes)
            }
        }
    }
}
