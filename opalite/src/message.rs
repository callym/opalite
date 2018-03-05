use std::sync::{ Arc, Mutex };
use fibers::sync::oneshot::{ self, Receiver };
use futures::{ Future, future::lazy };

pub trait Message: 'static + Send + Sized {
    fn send<H: 'static, R: 'static, E: 'static>(self, handler: Arc<Mutex<H>>) -> Box<Future<Item = R, Error = E> + Send> where H: Handler<Self, R, E> {
        Box::new(lazy(move || {
            let mut handler = handler.lock().unwrap();
            handler.send(self)
        }))
    }
}

pub trait Handler<M: 'static, R: 'static, E: 'static>: 'static + Send + Sized where M: Message {
    fn send(&mut self, message: M) -> Box<Future<Item = R, Error = E> + Send>;
}
