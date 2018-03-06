use std::mem;
use futures::{
    prelude::*,
    executor::ThreadPool,
    future::{ join_all, loop_fn, Loop },
};
use winit::{ EventsLoop, WindowBuilder };
use crate::ComponentStores;

pub struct GameLoop {
    component_stores: ComponentStores,
    executor: ThreadPool,
    initial_futures: Vec<Box<Future<Item = (), Error = Never> + Send>>,
    window_builder: Option<WindowBuilder>,
    running: bool,
}

impl GameLoop {
    pub fn new(title: &str, mut window_builder: WindowBuilder, component_stores: ComponentStores) -> Self {
        window_builder = window_builder.with_title(title);

        let executor = ThreadPool::new();

        Self {
            component_stores,
            executor,
            initial_futures: vec![],
            window_builder: Some(window_builder),
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
        let window_builder = self.window_builder.take().unwrap();
        let event_loop = EventsLoop::new();
        let _window = window_builder.build(&event_loop).unwrap();

        let event_loop = loop_fn(event_loop, |mut event_loop| {
            let mut result: Option<Result<_, ()>> = None;

            event_loop.poll_events(|event| {
                use winit::{ Event::*, WindowEvent::* };
                result = match event {
                    WindowEvent { event, .. } => match event {
                        Closed => Some(Ok(Loop::Break(()))),
                        _ => None,
                    },
                    _ => None,
                };
            });

            if result.is_none() {
                result = Some(Ok(Loop::Continue(event_loop)));
            }

            result.unwrap()
        }).map_err(|_| loop { });

        let initial_futures = mem::replace(&mut self.initial_futures, vec![]);
        let futures = join_all(initial_futures)
            .map(|_| ());

        self.running = true;

        let _ = self.executor.spawn(Box::new(futures));
        let _ = self.executor.run(event_loop);
    }
}
