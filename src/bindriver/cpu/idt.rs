extern crate x86_64;
use x86_64::structures::idt::*;
use ::bindriver::cpu::pic::PIC_1_OFFSET;
pub const TIMER_INTERRUPT_ID: u8 = PIC_1_OFFSET;

fn crack_locks() {
    unsafe { ::bindriver::serial::SERIAL1.force_unlock() }
    unsafe { ::bindriver::vga_buffer::WRITER.force_unlock() }
}

macro_rules! busy_intr_handler {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut ExceptionStackFrame) {
            crack_locks();
            debug!("Intterupt occured");
            debug!("Interrupt {}:\n{:?}", stringify!($name), stack_frame);
            hlt_loop();
        }
    };
}

macro_rules! busy_intr_handle_errcode {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut ExceptionStackFrame, err: u64) {
            crack_locks();
            debug!("Intterupt occured");
            debug!("Interrupt {} ({:#018x}):\n{:?}", stringify!($name), err, stack_frame);
            hlt_loop();
        }
    };
}

macro_rules! intr {
    ($idt:ident, $name:ident) => {
        unsafe {
            $idt.$name
                .set_handler_fn($name)
                .set_stack_index(::bindriver::cpu::gdt::INTR_IST_INDEX);
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
        intr!(idt, page_fault);
        intr!(idt, machine_check);
        idt[usize::from(TIMER_INTERRUPT_ID)]
            .set_handler_fn(timer_interrupt);
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

extern "x86-interrupt" fn breakpoint(stack_frame: &mut ExceptionStackFrame) {
    dump_stack_addr!();
    debug!("BREAKPOINT\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn double_fault(stack_frame: &mut ExceptionStackFrame, error_code: u64) {
    crack_locks();
    error!("Double Fault, Kernel Halting...");
    error!("Error: {:x}", error_code);
    vga_println!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    vga_println!("\n\nBUSY LOOPING CORE");
    hlt_loop();
}

extern "x86-interrupt" fn page_fault(
    stack_frame: &mut ExceptionStackFrame,
    error_code: PageFaultErrorCode,
) {
    let addr: usize;
    unsafe { asm!("mov rax, cr2":"={eax}"(addr)::"eax":"intel", "volatile") };
    use vmem::pagetable::Page;
    use vmem::{mapper::map_new, mapper::MapType, PhysAddr};
    crack_locks();
    let page = Page::containing_address(addr);
    let paddr = unsafe { PhysAddr::new_unchecked(page.start_address() as u64) };
    if !error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        if page.start_address() >= ::vmem::KSTACK_END
            && page.start_address() <= ::vmem::KSTACK_START
        {
            debug!("mapping kstack page to {:#018x}", page.start_address());
            map_new(paddr, MapType::Stack);
            return;
        } else if page.start_address() == ::vmem::KSTACK_GUARD {
            panic!("stack in kernel stack guard");
        } else if page.start_address() >= 0x000057ac00200000
            && page.start_address() <= 0x000057ac002ff000
        {
            map_new(paddr, MapType::Stack);
        } else {
            panic!("cannot map userspace yet: {:#018x}", page.start_address());
        }
    } else {
        debug!(
            "pagefault occured: {:#018x} => {:?} \n\n {:?}",
            page.start_address(),
            error_code,
            stack_frame
        );
        panic!("pagefault todo:");
    }
}

extern "x86-interrupt" fn timer_interrupt(
    stack_frame: &mut ExceptionStackFrame)
{
    crack_locks();
    debug!("timer interrupt");
    ::bindriver::cpu::pic::end_of_interrupt(TIMER_INTERRUPT_ID);
}

pub fn hlt_loop() -> ! {
    loop {
        ::x86_64::instructions::hlt();
    }
}