use std::mem;
use failure::Error;
use futures::{
    prelude::*,
    executor::ThreadPool,
    future::{ join_all, loop_fn, Loop },
    stream::poll_fn,
};
use glutin::{ self, EventsLoop };

pub struct GameLoop {
    executor: ThreadPool,
    initial_futures: Vec<Box<Future<Item = (), Error = Never>>>,
    running: bool,
}

impl GameLoop {
    pub fn new() -> Self {
        let executor = ThreadPool::new();

        Self {
            executor,
            initial_futures: vec![],
            running: false,
        }
    }

    pub fn spawn<F>(&mut self, future: F) where F: 'static + Future + Send {
        let future = future
            .map(|_| ())
            .map_err(|_| loop { });
        let future = Box::new(future);
        if self.running {
            let _ = self.executor.spawn(future);
        } else {
            self.initial_futures.push(future);
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventsLoop::new();
        let event_loop = loop_fn(event_loop, |mut event_loop| {
            let mut result: Result<_, ()> = Err(());
            event_loop.poll_events(|event| {
                if let glutin::Event::WindowEvent { event, .. } = event {
                    match event {
                        glutin::WindowEvent::Closed => result = Ok(Loop::Break(())),
                        _ => (),
                    }
                }
            });
            result = Ok(Loop::Continue(event_loop));
            result
        }).map_err(|_| loop { });

        let mut initial_futures = mem::replace(&mut self.initial_futures, vec![]);
        initial_futures.push(Box::new(event_loop));
        let initial_futures = join_all(initial_futures);

        self.running = true;

        let _ = self.executor.run(initial_futures);
    }
}
