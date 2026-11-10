//! The error type of this crate.

use std::error::Error;
use std::fmt;

/// One failed call into libopenmpt.
#[derive(Debug)]
pub struct OpenmptError {
    /// The operation that failed, such as `"load"`.
    pub op: &'static str,
    /// The input that the operation received.
    pub input: String,
    /// The error code from libopenmpt, when it reported one.
    pub code: Option<i32>,
    /// The message that libopenmpt gave, or a message from this crate.
    pub message: String,
    /// A hint that tells the caller how to correct the input.
    pub hint: Option<&'static str>,
    /// Log lines that libopenmpt wrote during the call.
    pub log: Vec<String>,
}

impl fmt::Display for OpenmptError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{} ({}): {}", self.op, self.input, self.message)?;
        if let Some(hint) = self.hint {
            write!(out, ". {hint}")?;
        }
        Ok(())
    }
}

impl Error for OpenmptError {}

/// The result type of this crate.
pub type Result<T> = std::result::Result<T, OpenmptError>;
