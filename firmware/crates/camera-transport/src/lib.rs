//! Hardware-independent camera transport, shared by the ARM and MIPS applications.
pub mod annexb;
pub mod microphone;
pub mod mqtt;
pub mod rtsp;
pub use camera_control::{motion_control, position3d};
pub use rtsp::audio::{AudioHub, Talkback};
pub use rtsp::hub::{Hub, VideoHubs, VideoLimits};
pub use rtsp::proto::Credentials;
pub use rtsp::server::{
    Config, ServerLimits, bind, bind_with_limits, serve, serve_with, serve_with_limits,
};
