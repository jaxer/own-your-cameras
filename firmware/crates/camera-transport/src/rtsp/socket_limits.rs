//! Socket memory is part of the 80 MiB media budget, not left to Linux autotuning.
use std::io;
use std::os::fd::AsRawFd;

pub(super) fn apply(socket: &impl AsRawFd, send: i32, receive: i32) -> io::Result<()> {
    for (option, requested) in [(libc::SO_SNDBUF, send), (libc::SO_RCVBUF, receive)] {
        // SAFETY: the socket is borrowed for this call; each option takes exactly one live i32.
        let rc = unsafe {
            libc::setsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                option,
                (&requested as *const i32).cast(),
                std::mem::size_of::<i32>() as _,
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut actual = 0i32;
        let mut length = std::mem::size_of::<i32>() as libc::socklen_t;
        // SAFETY: same live socket; correctly sized writable output and length pointers.
        let rc = unsafe {
            libc::getsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                option,
                (&mut actual as *mut i32).cast(),
                &mut length,
            )
        };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        // Linux reports twice the request for accounting overhead. Setting these options
        // locks out autotuning. Other supported hosts may report just the requested value.
        if length as usize != std::mem::size_of::<i32>() || actual <= 0 || actual > 2 * requested {
            return Err(io::Error::other("socket buffer exceeds media budget"));
        }
    }
    Ok(())
}

pub(super) fn listener(socket: &std::net::TcpListener, backlog: i32) -> io::Result<()> {
    // Limits set on a listening socket are inherited by queued connections, before accept.
    apply(socket, 32 * 1024, 16 * 1024)?;
    // SAFETY: live listening TCP socket; only its bounded pending-connection count changes.
    if unsafe { libc::listen(socket.as_raw_fd(), backlog) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_os_sockets_accept_the_tcp_and_udp_budgets() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        super::listener(&listener, 8).unwrap();
        let peer = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (socket, _) = listener.accept().unwrap();
        apply(&socket, 64 * 1024, 32 * 1024).unwrap();
        apply(&peer, 64 * 1024, 32 * 1024).unwrap();
        let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        apply(&udp, 16 * 1024, 8 * 1024).unwrap();
    }
}
