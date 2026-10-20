//! The moqspeak transport: a JSON control protocol on a WebSocket to the Worker, with voice and
//! screens published as Media over QUIC broadcasts on the relay the Worker hands out.

pub mod media;
mod session;
mod wire;

pub use session::Worker;
