mod client; // traits and impl for extending client builder and client
mod environment; // environment struct and logic
mod error; // package error handling
mod request; // traits and impl for extending request and requestbuilder
mod template; // templating responses

pub use client::*;
pub use environment::*;
pub use error::*;
pub use request::*;
pub use template::*;

pub mod config;
