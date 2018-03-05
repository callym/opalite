use failure::Error;
use fibers::{ Spawn, Executor, ThreadPoolExecutor };
use futures::Future;

pub struct GameLoop {
    executor: ThreadPoolExecutor,
}

impl GameLoop {
    pub fn new() -> Result<Self, Error> {
        let executor = ThreadPoolExecutor::new()?;

        Ok(Self {
            executor,
        })
    }

    pub fn spawn<F>(&mut self, future: F) where F: 'static + Future + Send {
        let future = future
            .map(|_| ())
            .map_err(|_| ());
        self.executor.spawn(future);
    }

    pub fn run_tick(&mut self) -> Result<(), Error> {
        self.executor.run_once().map_err(|err| err.into())
    }
}
