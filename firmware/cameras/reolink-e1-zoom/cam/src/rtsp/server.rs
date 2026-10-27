//! Preserve the platform runtime's heartbeat at the common RTSP listener.
use crate::runtime::{Beats, Progress};
pub use camera_transport::rtsp::server::{Config, bind, serve};
use std::{
    io,
    net::TcpListener,
    sync::{Arc, atomic::AtomicBool},
};

pub fn serve_with(
    listener: &TcpListener,
    config: Arc<Config>,
    stop: &AtomicBool,
    beats: Option<&Beats>,
) -> io::Result<()> {
    let progress = || {
        if let Some(beats) = beats {
            beats.beat(Progress::Rtsp);
        }
    };
    camera_transport::serve_with(listener, config, stop, Some(&progress))
}
