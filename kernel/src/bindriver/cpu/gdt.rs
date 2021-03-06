use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
//pub const SCHEDULER_IST_INDEX: u16 = 1;
pub const INTR_IST_INDEX: u16 = 2;


macro_rules! make_stack {
  ($size:expr) => {{
      const STACK_SIZE: usize = ($size);
      #[repr(align(4096))]
      struct StackContainer([u8; STACK_SIZE]);
      static mut STACK: StackContainer = StackContainer([0; STACK_SIZE]);
      let stack_start = VirtAddr::from_ptr(unsafe{&STACK});
      stack_start + STACK_SIZE
  }}
}
lazy_static! {
  static ref TSS: TaskStateSegment = {
    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
      make_stack!(4096 * 16)
    };
    /*tss.interrupt_stack_table[SCHEDULER_IST_INDEX as usize] = {
      make_stack!(8192)
    };*/
    tss.interrupt_stack_table[INTR_IST_INDEX as usize] = {
      make_stack!(4096 * 16)
    };
    tss
  };

  static ref GDT: (GlobalDescriptorTable, Selectors) = {
    let mut gdt = GlobalDescriptorTable::new();
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
    (gdt, Selectors { code_selector, tss_selector })
  };
}

struct Selectors {
  code_selector: SegmentSelector,
  tss_selector: SegmentSelector,
}

pub fn init() {
  use ::x86_64::instructions::segmentation::set_cs;
  use ::x86_64::instructions::tables::load_tss;
  GDT.0.load();
  unsafe {
    set_cs(GDT.1.code_selector);
    load_tss(GDT.1.tss_selector);
  }
}