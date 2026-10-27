use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub fn run() -> std::io::Result<()> {
    println!("mi360 userspace self-test");
    println!(
        "atomic32={} atomic64={}",
        cfg!(target_has_atomic = "32"),
        cfg!(target_has_atomic = "64")
    );
    println!("target_env_uclibc={}", cfg!(target_env = "uclibc"));
    let started = Instant::now();
    let count = Arc::new(AtomicU32::new(0));
    let worker_count = count.clone();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let worker = std::thread::Builder::new()
        .name("mi360-probe".into())
        .spawn(move || -> std::io::Result<()> {
            let until = Instant::now() + Duration::from_secs(3);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(pair) => break pair,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until =>
                    {
                        std::thread::yield_now()
                    }
                    Err(e) => return Err(e),
                }
            };
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            stream.set_write_timeout(Some(Duration::from_secs(2)))?;
            let mut data = [0_u8; 4];
            stream.read_exact(&mut data)?;
            stream.write_all(&data)?;
            worker_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })?;
    let mut client = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    client.set_write_timeout(Some(Duration::from_secs(2)))?;
    client.write_all(b"ping")?;
    let mut response = [0; 4];
    client.read_exact(&mut response)?;
    worker.join().expect("worker thread panic")?;
    assert_eq!(&response, b"ping");
    assert_eq!(count.load(Ordering::SeqCst), 1);
    println!(
        "thread_tcp_atomic_ok elapsed_us={}",
        started.elapsed().as_micros()
    );
    Ok(())
}
