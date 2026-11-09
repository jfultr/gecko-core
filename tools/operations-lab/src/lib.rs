//! Interactive lab stand for the Gecko Operation module (`0.1.0-dev`, GKO-68).
//!
//! Two terminal UIs in separate processes: a lab Gecko ([`server::Lab`]) with a
//! stub executor and fault controls, and a caller ([`client::LabClient`]). The
//! screens only call these controllers, so the controllers are tested without
//! a terminal. Test tool, not for production: it uses the permissive test
//! policy and the test hooks of `operations` and `operations-zenoh`.

pub mod client;
pub mod client_ui;
pub mod logbook;
pub mod server;
pub mod server_ui;
pub mod tui;
