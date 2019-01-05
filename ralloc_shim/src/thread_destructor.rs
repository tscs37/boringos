//! Thread destructors.
//!
//! This module supplies the ability to register destructors called upon thread exit.

pub use self::arch::*;

// BOS has no thread destructor
#[cfg(target_os = "boringos")]
pub mod arch {
    pub fn register(t: *mut u8, dtor: unsafe extern fn(*mut u8)) {
      import_symbol!(bos_log_trace, fn(&str));
      bos_log_trace("setting thread destructor");
        panic!()
    }
}