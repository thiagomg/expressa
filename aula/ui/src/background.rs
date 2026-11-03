//! Bounded workers shared by windows. Queued jobs can be cancelled; running
//! filesystem calls finish normally, and callers must validate their snapshots.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};

thread_local! {
    static CANCELLATION: std::cell::RefCell<Option<Arc<AtomicBool>>> = const { std::cell::RefCell::new(None) };
}
pub fn current_cancellation() -> Option<Arc<AtomicBool>> {
    CANCELLATION.with(|c| c.borrow().clone())
}

const IO_WORKERS: usize = 4;
const IO_QUEUE: usize = 32;
const ANALYSIS_WORKERS: usize = 2;
const ANALYSIS_QUEUE: usize = 16;
type Job = Box<dyn FnOnce() + Send>;

pub struct Pool {
    sender: SyncSender<Job>,
}
impl Pool {
    pub fn dispatch(
        &self,
        work: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Result<(), String> {
        self.sender
            .try_send(Box::new(move || {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("{e}"),
                    Err(_) => eprintln!("falha interna ao persistir as configurações"),
                }
            }))
            .map_err(|_| "fila de gravação local cheia; tente novamente".into())
    }
    fn new(workers: usize, capacity: usize, name: &str) -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Job>(capacity);
        let receiver = Arc::new(Mutex::new(receiver));
        for i in 0..workers {
            let receiver = receiver.clone();
            if std::thread::Builder::new()
                .name(format!("{name}-{i}"))
                .spawn(move || {
                    loop {
                        let job = receiver.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        match job {
                            Ok(job) => job(),
                            Err(_) => break,
                        }
                    }
                })
                .is_err()
            {
                break;
            }
        }
        Self { sender }
    }

    pub fn submit<T: Send + 'static>(
        &self,
        epoch: u64,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Reply<T> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let failure = sender.clone();
        let job = Box::new(move || {
            if flag.load(Ordering::Acquire) {
                return;
            }
            let previous = CANCELLATION.with(|c| c.replace(Some(flag.clone())));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                .unwrap_or_else(|_| Err("falha interna na operação em segundo plano".into()));
            CANCELLATION.with(|c| c.replace(previous));
            if !flag.load(Ordering::Acquire) {
                let _ = sender.send(result);
            }
        });
        if let Err(error) = self.sender.try_send(job) {
            let message = match error {
                TrySendError::Full(_) => "há muitas operações em andamento; tente novamente",
                TrySendError::Disconnected(_) => "o serviço em segundo plano foi encerrado",
            };
            let _ = failure.send(Err(message.into()));
        }
        Reply {
            epoch,
            receiver,
            cancelled,
            durable: false,
        }
    }
}

pub fn io() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| Pool::new(IO_WORKERS, IO_QUEUE, "aula-io"))
}
pub fn analysis() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| Pool::new(ANALYSIS_WORKERS, ANALYSIS_QUEUE, "aula-analysis"))
}
pub fn persistence() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| Pool::new(1, IO_QUEUE, "aula-persist"))
}

pub enum Poll<T> {
    Pending,
    Obsolete,
    Ready(Result<T, String>),
}
pub struct Reply<T> {
    epoch: u64,
    receiver: Receiver<Result<T, String>>,
    cancelled: Arc<AtomicBool>,
    durable: bool,
}
impl<T: Send + 'static> Reply<T> {
    pub fn keep_running(&mut self) {
        self.durable = true;
    }
    pub fn cancellation(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
    pub fn spawn(epoch: u64, work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Self {
        io().submit(epoch, work)
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn poll(&self, epoch: u64) -> Poll<T> {
        if self.epoch != epoch || self.cancelled.load(Ordering::Acquire) {
            self.cancel();
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
impl<T> Drop for Reply<T> {
    fn drop(&mut self) {
        if !self.durable {
            self.cancelled.store(true, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    fn wait<T: Send + 'static>(reply: &Reply<T>) -> Result<T, String> {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            match reply.poll(1) {
                Poll::Ready(r) => return r,
                Poll::Pending if Instant::now() < end => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                _ => panic!("worker did not complete"),
            }
        }
    }
    #[test]
    fn stale_replies_are_rejected_and_queued_cancellation_never_runs_work() {
        let pool = Pool::new(1, 1, "test-cancel");
        let (started, start) = std::sync::mpsc::channel();
        let (release, block) = std::sync::mpsc::channel();
        let first = pool.submit(1, move || {
            started.send(()).unwrap();
            block.recv().unwrap();
            Ok(())
        });
        start.recv_timeout(Duration::from_secs(5)).unwrap();
        let ran = Arc::new(AtomicBool::new(false));
        let flag = ran.clone();
        let queued = pool.submit(1, move || {
            flag.store(true, Ordering::Release);
            Ok(())
        });
        assert!(matches!(queued.poll(2), Poll::Obsolete));
        release.send(()).unwrap();
        wait(&first).unwrap();
        drop(pool);
        assert!(!ran.load(Ordering::Acquire));
    }
    #[test]
    fn admission_is_bounded_and_worker_survives_panics() {
        let pool = Pool::new(1, 1, "test-capacity");
        let (started, start) = std::sync::mpsc::channel();
        let (release, block) = std::sync::mpsc::channel();
        let first = pool.submit(1, move || {
            started.send(()).unwrap();
            block.recv().unwrap();
            Ok(())
        });
        start.recv_timeout(Duration::from_secs(5)).unwrap();
        let queued = pool.submit::<()>(1, || panic!("injected worker failure"));
        let rejected = pool.submit(1, || Ok(()));
        assert!(wait(&rejected).unwrap_err().contains("muitas operações"));
        release.send(()).unwrap();
        wait(&first).unwrap();
        assert!(wait(&queued).unwrap_err().contains("falha interna"));
        assert_eq!(wait(&pool.submit(1, || Ok(42))), Ok(42));
    }
}
