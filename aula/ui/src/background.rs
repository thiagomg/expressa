//! A worker reply belongs to the navigation that requested it.
use std::sync::mpsc::{Receiver, TryRecvError};

pub enum Poll<T> {
    Pending,
    Obsolete,
    Ready(Result<T, String>),
}

pub struct Reply<T> {
    epoch: u64,
    receiver: Receiver<Result<T, String>>,
}

impl<T: Send + 'static> Reply<T> {
    pub fn spawn(epoch: u64, work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(work());
        });
        Self { epoch, receiver }
    }

    pub fn poll(&self, epoch: u64) -> Poll<T> {
        if self.epoch != epoch {
            return Poll::Obsolete;
        }
        match self.receiver.try_recv() {
            Ok(result) => Poll::Ready(result),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => {
                Poll::Ready(Err("a operação em segundo plano foi interrompida".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_reply_from_previous_connection_is_discarded_even_on_failure() {
        for result in [Ok(vec!["old".to_string()]), Err("offline".into())] {
            let (sender, receiver) = std::sync::mpsc::channel();
            let reply = Reply { epoch: 1, receiver };
            assert!(matches!(reply.poll(1), Poll::Pending));
            sender.send(result).unwrap();
            assert!(matches!(reply.poll(2), Poll::Obsolete));
        }
    }
    #[test]
    fn current_connection_receives_lists_and_failures() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let reply = Reply { epoch: 3, receiver };
        sender.send(Ok(vec!["new".to_string()])).unwrap();
        assert!(matches!(reply.poll(3), Poll::Ready(Ok(names)) if names == ["new"]));
        sender.send(Err("offline".into())).unwrap();
        assert!(matches!(reply.poll(3), Poll::Ready(Err(e)) if e == "offline"));
    }
}
