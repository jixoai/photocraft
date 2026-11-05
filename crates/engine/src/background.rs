use std::io;

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
static WEB_READY: AtomicBool = AtomicBool::new(false);

pub fn available() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        true
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    {
        WEB_READY.load(Ordering::Acquire)
    }
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    {
        false
    }
}

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
pub fn enable_web_workers() {
    WEB_READY.store(true, Ordering::Release);
}

#[cfg(not(target_arch = "wasm32"))]
pub type Handle = std::thread::JoinHandle<()>;

#[cfg(target_arch = "wasm32")]
pub struct Handle {
    finished: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(target_arch = "wasm32")]
impl Handle {
    pub fn is_finished(&self) -> bool {
        self.finished.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn join(self) -> std::thread::Result<()> {
        if self.is_finished() { Ok(()) } else { Err(Box::new("worker has not finished")) }
    }
}

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> io::Result<Handle> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::Builder::new().name(name.into()).spawn(work)
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
    {
        if !available() {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "browser workers are not initialized"));
        }
        let _ = name;
        let finished = std::sync::Arc::new(AtomicBool::new(false));
        let done = finished.clone();
        rayon::spawn(move || {
            work();
            done.store(true, Ordering::Release);
        });
        Ok(Handle { finished })
    }
    #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
    {
        let _ = (name, work);
        Err(io::Error::new(io::ErrorKind::Unsupported, "this browser build has no workers"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn workers_deliver_shared_data_without_joining_a_running_task() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let worker = super::spawn("background-test", move || {
            sender.send(42).unwrap();
            wait.recv().unwrap();
        })
        .unwrap();
        assert_eq!(receiver.recv_timeout(std::time::Duration::from_secs(5)).unwrap(), 42);
        assert!(!worker.is_finished());
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(super::available());
    }
}
