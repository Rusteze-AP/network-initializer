use crossbeam::channel::{unbounded, Receiver, Sender};

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

impl<T> Default for Channel<T> {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Channel {
            sender: tx,
            receiver: rx,
        }
    }
}
