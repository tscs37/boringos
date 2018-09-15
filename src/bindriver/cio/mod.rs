
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
