use crossbeam::channel::{Receiver, Sender};

#[derive(Debug, Clone)]
pub struct Channel<T> {
    pub sender: Sender<T>,
    pub receiver: Receiver<T>,
}

impl<T> Channel<T> {
    pub fn new(sender: Sender<T>, receiver: Receiver<T>) -> Self {
        Channel { sender, receiver }
    }
}
