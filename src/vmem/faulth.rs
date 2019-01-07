
use x86_64::structures::idt::PageFaultErrorCode;
use crate::vmem::pagelist::PhysAddr;
use crate::vmem::{mapper::map_new, mapper::MapType, PAGE_SIZE};
use crate::vmem::pagetable::Page;

pub type PFHResult = Result<PFHOkResult, PFHErrResult>;

#[derive(Debug)]
pub enum PFHOkResult {
  Mapped,
}

#[derive(Debug)]
pub enum PFHErrResult {
  NoneError,
  InvalidAddress(usize),
}

impl From<core::option::NoneError> for PFHErrResult {
  fn from(f: core::option::NoneError) -> PFHErrResult { PFHErrResult::NoneError }
}

impl Into<PFHResult> for PFHErrResult {
  fn into(self) -> PFHResult {
    Err(self)
  }
}

impl Into<PFHResult> for PFHOkResult {
  fn into(self) -> PFHResult {
    Ok(self)
  }
}

pub fn handle(paddr: PhysAddr, error_code: PageFaultErrorCode) -> PFHResult {
  let caused_by_prot_violation = error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION);
  let caused_by_write = error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE);
  let caused_by_usermode = error_code.contains(PageFaultErrorCode::USER_MODE);
  let caused_by_instr_fetch = error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH);
  let caused_by_malformed_table = error_code.contains(PageFaultErrorCode::MALFORMED_TABLE);
  let page = paddr.into_page();
  let is_kstack = page.start_address() >= crate::vmem::KSTACK_END
      && page.start_address() <= crate::vmem::KSTACK_START + PAGE_SIZE;
  let is_ustack = page.start_address() >= crate::vmem::STACK_END
      && page.start_address() <= crate::vmem::STACK_START + PAGE_SIZE;
  let is_datapage = page.start_address() >= crate::vmem::DATA_START
      && page.start_address() <= crate::vmem::DATA_END;
  let is_codepage = page.start_address() >= crate::vmem::CODE_START
      && page.start_address() <= crate::vmem::CODE_END;

  let paddr = PhysAddr::new(page.start_address() as u64);
  let paddr = match paddr {
    Some(paddr) => paddr,
    None => { return PFHErrResult::InvalidAddress(page.start_address()).into() }
  };

  crate::vmem::pagetable::ActivePageTable::dump(&page);
  if caused_by_malformed_table {
      error!("page table malformed at {}", paddr);
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
          PFHOkResult::Mapped.into()
      } else if is_ustack {
          if caused_by_instr_fetch {
              //TODO: kill task instead
              panic!("task attempted to run instruction from stack")
          }
          trace!("page fault in user stack, mapping new pages");
          handle_ustack(paddr, page)
      } else if page.start_address() == crate::vmem::KSTACK_GUARD {
          panic!("stack in kernel stack guard");
      } else if is_codepage || is_datapage {
          if caused_by_instr_fetch {
              // Doesn't work?
              panic!("task could not execute in executable memory");
          }
          if crate::kinfo_mut().mapping_task_image(None) {
            handle_new_umemory(paddr, page, is_codepage, is_datapage)
          } else if is_datapage {
            handle_running_umemory(paddr, page)
          } else {
            panic!("tried to access bss or code memory outside mapping zone: {}, Data={}, Code={}", paddr, is_datapage, is_codepage);
          }
      } else {
          panic!("cannot map: {:#018x}", page.start_address());
      }
  } else {
      if is_kstack || is_ustack {
        error!("protection violation in stack area");
      }
      if is_codepage {
        error!("protection violation in code area");
      }
      if is_datapage {
        error!("protection violation in data memory");
      }
      error!(
        "uncovered pagefault occured: {:#018x} => ({:x}) {:?} \n\n",
        page.start_address(),
        error_code,
        error_code
      );
      panic!("todo: kill task is possible");
  }
}

fn handle_new_umemory(paddr: PhysAddr, page: Page, is_codepage: bool, is_datapage: bool) -> PFHResult {
  //TODO: check paging mode for paging new task
  //TODO: check if zero page touched
  trace!("checking if the kernel touched memory correctly");
  let expected_paddr = {
    if is_codepage {
      PhysAddr::new_usize_or_abort(
      crate::vmem::CODE_START
          + (PAGE_SIZE
              * (crate::kinfo().get_code_memory_ref_size())),
      )
    } else if is_datapage {
      PhysAddr::new_usize_or_abort(
      crate::vmem::DATA_START
          + (PAGE_SIZE
              * (crate::kinfo().get_data_memory_ref_size())),
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
    "mapped new code or data memory, notifying kernel for page {}<->{}",
    new_page, paddr
  );
  if is_codepage {
    crate::kinfo_mut().add_code_page(new_page);
  } else if is_datapage {
    crate::kinfo_mut().add_data_page(new_page);
  } else {
    panic!("Neither BSS or Code page in BSS or Code only path of page fault");
  }
  PFHOkResult::Mapped.into()
}

fn handle_running_umemory(paddr: PhysAddr, page: Page) -> PFHResult {
  //TODO: check paging mode for paging new task
  //TODO: check if zero page touched
  trace!("checking if the kernel touched memory correctly");
  let expected_paddr = PhysAddr::new_usize_or_abort(
    crate::vmem::DATA_START
        + (PAGE_SIZE
            * (crate::kinfo().get_data_memory_ref_size())),
    );
  if expected_paddr != paddr {
    error!(
      "wanted task to touch {} but it touched {}",
      expected_paddr, paddr
    );
    panic!("task touched data memory early, that's nasty");
  }
  let new_page = map_new(paddr, MapType::Data);
  trace!(
    "mapped new data memory, notifying kernel for page {}<->{}",
    new_page, paddr
  );
  crate::kinfo_mut().add_data_page(new_page);
  PFHOkResult::Mapped.into()
}

fn handle_ustack(paddr: PhysAddr, page: Page) -> PFHResult {
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
  let diff_pages = 
    (laststack_paddr.as_usize() - paddr.as_usize()) / PAGE_SIZE + 1;
  if diff_pages > 1 {
    // sometimes rust jumps a page or two ahead, we yell at it but allow it
    // TODO: include page jumping in task statistics and kill process if it does this too often
    warn!("task jumped {} pages instead of 1, wanted {} but got {}", 
      diff_pages, laststack_paddr, paddr);
    for x in 0..diff_pages {
      let tar_addr = PhysAddr::new_usize_or_abort(
        laststack_paddr.as_usize() - x * PAGE_SIZE);
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
  PFHOkResult::Mapped.into()
}