//! Testing without the network: a fake server that answers every operation of the API
//! (`testing` feature).

mod routes;
mod server;

pub use routes::ROUTES;
pub use server::{FakeServer, Recorded, Reply, Route, StreamReply};
