extern crate pic8259_simple;

use self::pic8259_simple::ChainedPics;
use spin;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
  spin::Mutex::new(unsafe { 
    ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
  });

pub fn init() {
  unsafe { PICS.lock().initialize(); }
  //::x86_64::instructions::interrupts::enable();
}

pub fn end_of_interrupt(id: u8) {
  unsafe { PICS.lock().notify_end_of_interrupt(id) }
}