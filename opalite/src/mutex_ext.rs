use std::sync::{ Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard };

#[derive(Debug, Clone)]
pub struct RLock<T>(pub(crate) Arc<RwLock<T>>);

impl<T> RLock<T> {
    pub fn new(data: T) -> Self {
        let lock = Arc::new(RwLock::new(data));
        RLock(lock)
    }

    pub fn read(&self) -> LockResult<RwLockReadGuard<T>> {
        self.0.read()
    }
}

#[derive(Debug, Clone)]
pub struct WLock<T>(pub(crate) Arc<RwLock<T>>);

impl<T> WLock<T> {
    pub fn new(data: T) -> Self {
        let lock = Arc::new(RwLock::new(data));
        WLock(lock)
    }

    pub fn get_reader(&self) -> RLock<T> {
        RLock(self.0.clone())
    }

    pub fn read(&self) -> LockResult<RwLockReadGuard<T>> {
        self.0.read()
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<T>> {
        self.0.write()
    }
}
