use crate::bindriver::cpu::pic::PIC_1_OFFSET;
use x86_64::structures::idt::*;
pub const TIMER_INTERRUPT_ID: u8 = PIC_1_OFFSET;

fn crack_locks() {
    unsafe { crate::bindriver::serial::SERIAL1.force_unlock() }
    unsafe { crate::bindriver::vga_buffer::WRITER.force_unlock() }
}

macro_rules! busy_intr_handler {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut ExceptionStackFrame) {
            crack_locks();
            debug!("Interrupt {}:\n{:?}", stringify!($name), stack_frame);
            hlt_cpu!();
        }
    };
}

macro_rules! busy_intr_handle_errcode {
    ($name:ident) => {
        extern "x86-interrupt" fn $name(stack_frame: &mut ExceptionStackFrame, err: u64) {
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
            extern "x86-interrupt" fn(&mut ExceptionStackFrame, u64)
            = page_fault;
        let page_fault: 
            extern "x86-interrupt" fn(&mut ExceptionStackFrame, PageFaultErrorCode)
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
    hlt_cpu!();
}

extern "x86-interrupt" fn page_fault(
    stack_frame: &mut ExceptionStackFrame,
    error_code: u64,
) {
    // LLVM fucks up the stack alignment, so we unfuck it and force the value through a sensible place
    {
        unsafe{asm!("sub rsp, 8
        sub rbp, 8"::::"intel", "volatile")};
    }
    let error_code = error_code.clone();
    {
        unsafe{asm!("add rsp, 8
        add rbp, 8"::::"intel", "volatile")};
    }
    let error_code = PageFaultErrorCode::from_bits(error_code)
        .expect(&format!("error_code has reserved bits set: {:#018x}", error_code))
        .clone();
    debug!("checking page fault error, code: {:08x}", error_code);
    let addr: usize;
    unsafe {
        asm!("
        mov rax, cr2
        ":"={rax}"(addr)::"rax", "memory":"intel", "volatile")
    };
    debug!("cr2 register says it was {:#018x}", addr);

    let caused_by_prot_violation = error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION);
    let caused_by_write = error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE);
    let caused_by_usermode = error_code.contains(PageFaultErrorCode::USER_MODE);
    let caused_by_instr_fetch = error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH);
    let caused_by_malformed_table = false && error_code.contains(PageFaultErrorCode::MALFORMED_TABLE);

    use crate::vmem::pagetable::Page;
    use crate::vmem::{mapper::map_new, mapper::MapType, PhysAddr, PAGE_SIZE};
    let page = Page::containing_address(addr);
    let paddr = unsafe { PhysAddr::new_unchecked(page.start_address() as u64) };
    let is_kstack = page.start_address() >= crate::vmem::KSTACK_END
        && page.start_address() <= crate::vmem::KSTACK_START + PAGE_SIZE;
    let is_ustack = page.start_address() >= crate::vmem::STACK_END
        && page.start_address() <= crate::vmem::STACK_START + PAGE_SIZE;
    let is_bsspage = page.start_address() >= crate::vmem::BSS_START
        && page.start_address() <= crate::vmem::BSS_END;
    let is_codepage = page.start_address() >= crate::vmem::CODE_START
        && page.start_address() <= crate::vmem::CODE_END;
    if page.start_address() == 0xfffffffffffff000 || page.start_address() == 0 {
        panic!(
            "critical page fault in page table area or zero memory: {:#018x}",
            page.start_address()
        );
    }
    crate::vmem::pagetable::ActivePageTable::dump(&page);
    if caused_by_malformed_table {
        error!("page table malformed at {:#018x}", addr);
        panic!()
    }
    if page.start_address() > crate::vmem::PAGE_TABLE_LO {
        error!("page fault should not occur in page table area");
        panic!();
    }
    if !caused_by_prot_violation {
        if is_kstack {
            if caused_by_instr_fetch {
                panic!("kernel attempted to run instruction from stack");
            }
            debug!("mapping kstack page to {:#018x}", page.start_address());
            map_new(paddr, MapType::Stack);
            //TODO: adjust kernel stack size
            debug!("mapped, returning...");
            return;
        } else if is_ustack {
            if caused_by_instr_fetch {
                //TODO: kill task instead
                panic!("task attempted to run instruction from stack")
            }
            trace!("checking if the task touched stack correctly");
            let stack_size_org = crate::kinfo().get_stack_memory_ref_size() + 1 - 2;
            let stack_size_org = stack_size_org + 1; // Adjust for 0-base
            let stack_size = stack_size_org - 2; // Allow touching up to 2 pages early
            let expected_paddr = PhysAddr::new_usize_or_abort(
                crate::vmem::STACK_START
                    - (PAGE_SIZE
                        * (stack_size)),
            );
            let laststack_paddr = PhysAddr::new_usize_or_abort(
                crate::vmem::STACK_START
                    - (PAGE_SIZE
                        * (stack_size_org)),
            );
            if expected_paddr <= paddr {
                error!(
                    "wanted task to touch {} but it touched {}",
                    laststack_paddr, paddr
                );
                panic!("task touched stack memory early, that's nasty");
            }
            let diff_pages = PhysAddr::new_usize_or_abort(
                expected_paddr.as_usize() - paddr.as_usize()).as_usize() / PAGE_SIZE + 1;
            if diff_pages > 1 {
                // sometimes rust jumps a page or two ahead, we yell at it but allow it
                // TODO: include page jumping in task statistics and kill process if it does this too often
                warn!("task jumped {} pages instead of 1, wanted {} but got {}", 
                    diff_pages, laststack_paddr, paddr);
                for x in 0..diff_pages {
                    let tar_addr = PhysAddr::new_usize_or_abort(
                        laststack_paddr.as_usize() + x * PAGE_SIZE);
                    trace!("mapping user stack page to {:#018x}", tar_addr.as_u64());
                    let new_page = map_new(tar_addr, MapType::Stack);
                    crate::kinfo_mut().add_stack_page(new_page);
                }
            } else if diff_pages == 1 {
                trace!("mapping user stack page to {:#018x}", page.start_address());
                let new_page = map_new(paddr, MapType::Stack);
                crate::kinfo_mut().add_stack_page(new_page);
            } else {
                panic!("page fault on supposedly mapped stack");
            }
            return;
        } else if page.start_address() == crate::vmem::KSTACK_GUARD {
            panic!("stack in kernel stack guard");
        } else if is_codepage || is_bsspage {
            if caused_by_instr_fetch {
                // Doesn't work?
                panic!("task could not execute in executable memory");
            }
            //TODO: check paging mode for paging new task
            //TODO: check if zero page touched
            trace!("page fault in user code or bss memory, checking if kernel is creating new task");
            if crate::kinfo_mut().mapping_task_image(None) {
                trace!("checking if the kernel touched memory correctly");
                let expected_paddr = {
                    if is_codepage {
                        PhysAddr::new_usize_or_abort(
                        crate::vmem::CODE_START
                            + (PAGE_SIZE
                                * (crate::kinfo().get_code_memory_ref_size())),
                        )
                    } else if is_bsspage {
                        PhysAddr::new_usize_or_abort(
                        crate::vmem::BSS_START
                            + (PAGE_SIZE
                                * (crate::kinfo().get_bss_memory_ref_size())),
                        )
                    } else {
                        panic!("Neither BSS or Code page in BSS or Code only path of page fault");
                    }
                };
                if expected_paddr != paddr {
                    error!(
                        "wanted kernel to touch {} but it touched {}",
                        expected_paddr, paddr
                    );
                    panic!("kernel touched code memory early, that's nasty");
                }
                let new_page = map_new(paddr, MapType::Data);
                trace!(
                    "mapped new code memory, notifying kernel for page {}<->{}",
                    new_page, paddr
                );
                if is_codepage {
                    crate::kinfo_mut().add_code_page(new_page);
                } else if is_bsspage {
                    crate::kinfo_mut().add_bss_page(new_page);
                } else {
                    panic!("Neither BSS or Code page in BSS or Code only path of page fault");
                }
                return;
            } else {
                panic!("tried to access bss or code memory outside mapping zone: {}, BSS={}, Code={}", paddr, is_bsspage, is_codepage);
            }
        } else {
            panic!("cannot map: {:#018x}", page.start_address());
        }
    } else {
        if paddr.as_usize() > crate::vmem::pagetable::LOW_PAGE_TABLE {
            warn!("page fault in page table area, checking if mapped...");
            if crate::vmem::mapper::is_mapped(paddr) {
                panic!("page fault in mapped page table");
            } else {
                panic!("page fault in unmapped page table");
                //map_new(paddr, MapType::Data);
                //return;
            }
        }
        if is_kstack || is_ustack {
            error!("protection violation in stack area");
        }
        error!(
            "uncovered pagefault occured: {:#018x} => ({:x}) {:?} \n\n",
            page.start_address(),
            error_code,
            error_code
        );
        if is_kstack {
            panic!("prot violation in kernel");
        }
        panic!("pagefault todo:");
    }
}

extern "x86-interrupt" fn timer_interrupt(_stack_frame: &mut ExceptionStackFrame) {
    trace!("timer interrupt");
    //TODO: dispatch all registered event handlers
    crate::bindriver::cpu::pic::end_of_interrupt(TIMER_INTERRUPT_ID);
}
