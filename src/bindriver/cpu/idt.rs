

extern crate x86_64;
use x86_64::structures::idt::*;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.page_fault.set_handler_fn(pagefault_handler);
        idt
    };
}

pub fn init_idt() {
  IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut ExceptionStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame, _error_code: u64) {
    println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    println!("\n\nBUSY LOOPING CORE");
    loop {}
}

extern "x86-interrupt" fn pagefault_handler(
    stack_frame: &mut ExceptionStackFrame, error_code: PageFaultErrorCode) {
    debug!("pagefault occured: {:?} \n\n {:?}", error_code, stack_frame);
    loop{}
}