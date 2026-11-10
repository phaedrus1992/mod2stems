//! Safe Rust wrapper over libopenmpt and its interactive extension interface.

// Every call into the C library is unsafe, and the wrapper exists to keep that
// unsafe code in this crate. Each block carries a `SAFETY` comment that states
// the precondition it relies on.
#![expect(
    unsafe_code,
    reason = "this crate is the safe boundary over the C library"
)]

mod cstr;
mod error;
mod info;
mod initial_ctl;
mod module;

pub use error::OpenmptError;
pub use error::Result;
pub use info::LibraryInfo;
pub use info::library_info;
pub use initial_ctl::InitialCtl;
pub use module::Module;
