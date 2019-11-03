use crate::bindriver::cpu::pic::PIC_1_OFFSET;
use x86_64::structures::idt::*;
use x86_64::VirtAddr;
pub const TIMER_INTERRUPT_ID: u8 = PIC_1_OFFSET;
use crate::*;

fn crack_locks() {
    unsafe { crate::bindriver::serial::SERIAL1.force_unlock() }
    #[cfg(feature="vga")]
    unsafe { crate::bindriver::vga_buffer::WRITER.force_unlock() }
}

macro_rules! busy_intr_handler {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut InterruptStackFrame) {
            crack_locks();
            debug!("Interrupt {}:\n{:?}", stringify!($name), stack_frame);
            hlt_cpu!();
        }
    };
}

macro_rules! busy_intr_handle_errcode {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut InterruptStackFrame, err: u64) {
            crack_locks();
            debug!(
                "Interrupt {} ({:#018x}):\n{:?}",
                stringify!($name),
                err,
                stack_frame
            );
            hlt_cpu!();
        }
    };
}

macro_rules! intr {
    ($idt:ident, $name:ident) => {
        unsafe {
            $idt.$name
                .set_handler_fn($name)
                .set_stack_index(crate::bindriver::cpu::gdt::INTR_IST_INDEX);
        }
    };
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        intr!(idt, divide_by_zero);
        intr!(idt, non_maskable_interrupt);
        intr!(idt, breakpoint);
        intr!(idt, overflow);
        intr!(idt, bound_range_exceeded);
        intr!(idt, invalid_opcode);
        intr!(idt, device_not_available);
        intr!(idt, double_fault);
        intr!(idt, invalid_tss);
        intr!(idt, segment_not_present);
        intr!(idt, stack_segment_fault);
        intr!(idt, general_protection_fault);
        // yes, this is necessary
        // no, you can't touch this
        // no, I don't want this either
        // yes, it breaks if you look at it funny
        // stop looking at it
        let page_fault: 
            extern "x86-interrupt" fn(&mut InterruptStackFrame, u64)
            = page_fault;
        let page_fault: 
            extern "x86-interrupt" fn(&mut InterruptStackFrame, PageFaultErrorCode)
            = unsafe{core::mem::transmute(page_fault)};
        unsafe{idt.page_fault
            .set_handler_fn(page_fault)
            .set_stack_index(crate::bindriver::cpu::gdt::INTR_IST_INDEX)};
        intr!(idt, machine_check);
        idt[usize::from(TIMER_INTERRUPT_ID)].set_handler_fn(timer_interrupt);
        idt
    };
}

pub fn init() {
    IDT.load();
}

busy_intr_handler!(divide_by_zero);
busy_intr_handler!(non_maskable_interrupt);
busy_intr_handler!(overflow);
busy_intr_handler!(bound_range_exceeded);
busy_intr_handler!(invalid_opcode);
busy_intr_handler!(device_not_available);
busy_intr_handler!(machine_check);
busy_intr_handle_errcode!(invalid_tss);
busy_intr_handle_errcode!(segment_not_present);
busy_intr_handle_errcode!(stack_segment_fault);
busy_intr_handle_errcode!(general_protection_fault);

extern "x86-interrupt" fn breakpoint(stack_frame: &mut InterruptStackFrame) {
    debug!("BREAKPOINT\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn double_fault(stack_frame: &mut InterruptStackFrame, error_code: u64) {
    crack_locks();
    error!("Double Fault, Kernel Halting...");
    error!("Error: {:x}", error_code);
    #[cfg(feature = "vga")]
    vga_println!("EXCEPTION: DOUBLE FAULT\n{:#?}\n\nBUSY LOOPING CORE", stack_frame);
    hlt_cpu!();
}

use super::PageFaultContext;

extern "x86-interrupt" fn page_fault(
    stack_frame: &mut InterruptStackFrame,
    error_code: u64,
) {
    let error_code = PageFaultErrorCode::from_bits(error_code)
        .expect("invalid PFEC");
    debug!("checking page fault error, code: {:08x}", error_code);
    let addr: usize;
    unsafe {
        asm!("
        mov rax, cr2
        ":"={rax}"(addr)::"rax", "memory":"intel", "volatile")
    };
    debug!("fault addr: {:#018x}", addr);
    debug!("instr addr: {:#018x}", stack_frame.instruction_pointer.as_u64());

    assert!(addr >= crate::vmem::KERNEL_START, "valid memory must be above {:#018x}", crate::vmem::KERNEL_START);
    let vaddr = VirtAddr::new(addr.try_into().unwrap());
    let pfc = PageFaultContext::new(vaddr, error_code, stack_frame.instruction_pointer);
    match crate::vmem::faulth::handle(pfc) {
        Ok(res) => debug!("Handler returned Ok: {:?}", res),
        Err(res) => panic!("Handler returned Error: {:?}", res)
    }
    
}

extern "x86-interrupt" fn timer_interrupt(_stack_frame: &mut InterruptStackFrame) {
    trace!("timer interrupt");
    //TODO: dispatch all registered event handlers
    crate::bindriver::cpu::pic::end_of_interrupt(TIMER_INTERRUPT_ID);
}
