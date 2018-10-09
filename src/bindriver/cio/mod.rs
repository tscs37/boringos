
macro_rules! print {
    ($($arg:tt)*) => {
      $crate::bindriver::vga_buffer::print(format_args!($($arg)*));
      $crate::bindriver::serial::print(format_args!($($arg)*))
    };
}

macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! print_green {
    ($($arg:tt)*) => {
      $crate::bindriver::vga_buffer::print_green(format_args!($($arg)*));
      $crate::bindriver::serial::print(format_args!($($arg)*))
    };
}

macro_rules! print_red {
    ($($arg:tt)*) => {
      $crate::bindriver::vga_buffer::print_red(format_args!($($arg)*));
      $crate::bindriver::serial::print(format_args!($($arg)*))
    };
}

macro_rules! debug {
    ($fmt:expr) => {
      $crate::bindriver::serial::print(format_args!("    debug: {}\n", $fmt))
    };
    ($fmt:expr, $($arg:tt)*) => {
      $crate::bindriver::serial::print(format_args!(concat!("    debug: ", $fmt, "\n"), $($arg)*))
    };
}

//TODO: add more debug levels:
/* - verbose (debug+memory subsystem)
 * - debug (without memory subsystem)
 * - info
 * - warn (return no error, not critical)
 * - error (return error)
 * - critical (run coredump then hang)
 * - panic (no coredump, just hang)
 */

//TODO: allow hooks to get other subsystems to send
//      and receive debug messages