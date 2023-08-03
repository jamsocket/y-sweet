#[derive(Clone)]
pub struct Threadless<T>(pub T);

#[cfg(target_arch = "wasm32")]
unsafe impl<T> Send for Threadless<T> {}

#[cfg(target_arch = "wasm32")]
unsafe impl<T> Sync for Threadless<T> {}
