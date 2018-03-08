use std::ops;
use std::sync::{ Arc, Mutex, MutexGuard, mpsc };
use owning_ref::MutexGuardRefMut;
use specs::System;

pub trait Message { }

pub struct MessageQueue;

impl MessageQueue {
    pub fn new<M: Message>() -> (MessageSender<M>, MessageReceiver<M>) {
        let (sender, receiver) = mpsc::channel();

        (MessageSender::new(sender), MessageReceiver::new(receiver))
    }
}

#[derive(Clone)]
pub struct MessageSender<M: Message>(Arc<Mutex<mpsc::Sender<M>>>);

impl<M: Message> MessageSender<M> {
    fn new(queue: mpsc::Sender<M>) -> Self {
        MessageSender(Arc::new(Mutex::new(queue)))
    }

    pub fn send(&mut self, message: M) {
        let sender = self.0.lock().unwrap();
        sender.send(message).unwrap();
    }
}

pub struct MessageIter<'a, M: Message + 'a> {
    guard: MutexGuardRefMut<'a, mpsc::Receiver<M>>,
}

impl<'a, M: Message + 'a> MessageIter<'a, M> {
    fn new(guard: MutexGuard<'a, mpsc::Receiver<M>>) -> Self {
        let guard = MutexGuardRefMut::new(guard);
        Self { guard }
    }
}

impl<'a, M: Message + 'a> Iterator for MessageIter<'a, M> {
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        self.guard.try_recv().ok()
    }
}

pub struct MessageReceiver<M: Message>(Arc<Mutex<mpsc::Receiver<M>>>);

impl<M: Message> MessageReceiver<M> {
    fn new(queue: mpsc::Receiver<M>) -> Self {
        MessageReceiver(Arc::new(Mutex::new(queue)))
    }

    pub fn next_message(&mut self) -> Option<M> {
        self.0.lock().unwrap().try_recv().ok()
    }

    pub fn messages<'a, 'b: 'a>(&'b mut self) -> MessageIter<'a, M> {
        MessageIter::new(self.0.lock().unwrap())
    }
}

pub trait Shard<'a>: System<'a> {
    type Message: Message;

    fn sender(&self) -> MessageSender<Self::Message>;
}
