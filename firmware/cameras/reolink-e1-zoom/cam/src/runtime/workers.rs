//! Owned service threads. Early return, error and unwind are failures of the enclosing service.
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};

type Worker = (&'static str, JoinHandle<io::Result<()>>);

pub struct Workers<'a> {
    stop: &'a AtomicBool,
    threads: Vec<Worker>,
    /// First worker to end, recorded before its held guard drops (so before any lease reports it).
    exited: Arc<OnceLock<&'static str>>,
}

/// Records the exit (return or unwind), then drops the held guard: `Drop` runs before fields drop.
struct Exit<G> {
    name: &'static str,
    exited: Arc<OnceLock<&'static str>>,
    _guard: G,
}

impl<G> Drop for Exit<G> {
    fn drop(&mut self) {
        let _ = self.exited.set(self.name);
    }
}

impl<'a> Workers<'a> {
    pub fn new(stop: &'a AtomicBool) -> Self {
        Self {
            stop,
            threads: Vec::new(),
            exited: Arc::default(),
        }
    }

    /// Spawn failure is returned to the caller; previously started threads remain owned.
    pub fn spawn(
        &mut self,
        name: &'static str,
        run: impl FnOnce() -> io::Result<()> + Send + 'static,
    ) -> io::Result<()> {
        self.spawn_holding(name, (), run)
    }

    /// Like `spawn`, but `guard` lives until the worker has ended and `check` can name it. Pass a
    /// `WorkerLease` here so whoever sees the lease drop also gets this worker's exit from `check`.
    pub fn spawn_holding(
        &mut self,
        name: &'static str,
        guard: impl Send + 'static,
        run: impl FnOnce() -> io::Result<()> + Send + 'static,
    ) -> io::Result<()> {
        let exit = Exit {
            name,
            exited: Arc::clone(&self.exited),
            _guard: guard,
        };
        let handle = thread::Builder::new().name(name.into()).spawn(move || {
            let _exit = exit;
            run()
        })?;
        self.threads.push((name, handle));
        Ok(())
    }

    /// Called by the lifecycle owner between media receive batches. A blocked media poll still
    /// needs the independent production watchdog; this check cannot detect a stalled caller.
    pub fn check(&mut self) -> io::Result<()> {
        if !self.stop.load(Ordering::Relaxed)
            && let Some(i) = self
                .threads
                .iter()
                .position(|(name, worker)| worker.is_finished() || self.exited.get() == Some(name))
        {
            self.stop.store(true, Ordering::Release);
            let (name, worker) = self.threads.remove(i);
            joined(name, worker)?;
            return Err(io::Error::other(format!("{name} exited unexpectedly")));
        }
        Ok(())
    }

    /// Stop and join all workers, returning all failures only after every join. Do not time out
    /// and tear down hardware under a stuck worker: leave the graph alive for watchdog recovery.
    pub fn shutdown(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        let mut errors = Vec::new();
        for (name, worker) in self.threads.drain(..) {
            if let Err(error) = joined(name, worker) {
                errors.push(error.to_string());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(errors.join("; ")))
        }
    }
}

impl Drop for Workers<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            eprintln!("service cleanup: {error}");
        }
    }
}

fn joined(name: &str, worker: JoinHandle<io::Result<()>>) -> io::Result<()> {
    match worker.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(io::Error::other(format!("{name}: {error}"))),
        Err(_) => Err(io::Error::other(format!("{name} panicked"))),
    }
}
