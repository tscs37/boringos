pub mod helper;
pub mod buffer;

pub struct Writer {
    pub column_position: usize,
    pub color_code: helper::ColorCode,
    pub buffer: &'static mut buffer::Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= buffer::BUFFER_WIDTH {
                    self.new_line();
                }

                let row = buffer::BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(buffer::ScreenChar {
                    ascii_character: byte,
                    color_code: color_code,
                });
                self.column_position += 1;
            }
        }
    }
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20...0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }
    fn clear_row(&mut self, row: usize) {
      let blank = buffer::ScreenChar{
        ascii_character: b' ',
        color_code: self.color_code,
      };
      for col in 0..buffer::BUFFER_WIDTH {
        self.buffer.chars[row][col].write(blank);
      }
    }
    fn new_line(&mut self) {
      for row in 1..buffer::BUFFER_HEIGHT {
        for col in 0..buffer::BUFFER_WIDTH {
          let character = self.buffer.chars[row][col].read();
          self.buffer.chars[row - 1][col].write(character);
        }
      }
      self.clear_row(buffer::BUFFER_HEIGHT - 1);
      self.column_position = 0;
    }
}

use core::fmt;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

use spin::Mutex;

lazy_static! {
  pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
      column_position: 0,
      color_code: helper::ColorCode::new(helper::Color::LightGray, helper::Color::Black),
      buffer: unsafe { &mut *(0xb8000 as *mut buffer::Buffer) },
  });
}

pub fn print(args: fmt::Arguments) {
  use core::fmt::Write;
  WRITER.lock().write_fmt(args).unwrap();
}

pub fn print_green(args: fmt::Arguments) {
  use core::fmt::Write;
  let mut w = WRITER.lock();
  let old_color = w.color_code;
  w.color_code = helper::ColorCode::new(helper::Color::Green, helper::Color::Black);
  w.write_fmt(args).unwrap();
  w.color_code = old_color;
}

pub fn print_red(args: fmt::Arguments) {
  use core::fmt::Write;
  let mut w = WRITER.lock();
  let old_color = w.color_code;
  w.color_code = helper::ColorCode::new(helper::Color::Red, helper::Color::Black);
  w.write_fmt(args).unwrap();
  w.color_code = old_color;
}