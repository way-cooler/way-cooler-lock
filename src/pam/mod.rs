//! Wrapper around authentication code written in C.
//!
//! NOTE The reason such a security-sensitive module is written in C instead of
//! Rust is because there are no good PAM crates available as of May 2017.
//! If this changes in the future, this module should be removed in favor
//! of using more battle-tested code.

use libc::c_char;

extern "C" {
    /// Checks to see if the username and password are valid through PAM.
    ///
    /// Both strings should be null-terminated and non-null.
    pub fn check_auth(username: *const c_char,
                      password: *const c_char)
                      -> bool;
}
