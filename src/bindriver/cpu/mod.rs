pub mod gdt;
pub mod idt;
pub mod pic;
pub mod rng;
use raw_cpuid::{CpuId, FeatureInfo};
use x86_64::VirtAddr;
use x86_64::structures::idt::PageFaultErrorCode;

fn cpuid() -> CpuId {
  CpuId::new()
}

fn feature_info() -> Option<FeatureInfo> {
  cpuid().get_feature_info()
}

pub fn has_rdrand() -> bool {
  if let Some(info) = feature_info() {
    info.has_rdrand()
  } else {
    false
  }
}

pub fn has_acpi() -> bool {
  if let Some(info) = feature_info() {
    info.has_acpi()
  } else {
    false
  }
}

pub fn enable_nxe_bit() {
  unsafe {
    use x86_64::registers::model_specific::Efer;
    let mut flags = Efer::read_raw();
    flags |= 1 << 11;
    Efer::write_raw(flags);
  }
}

pub struct PageFaultContext {
  // Page Fault Address
  fault_address: VirtAddr,
  // Error Code
  error_code: PageFaultErrorCode,
  // Address that caused the fault
  instr_address: VirtAddr,
}

impl PageFaultContext {
  pub fn new(fault_address: VirtAddr, error_code: PageFaultErrorCode, instr_address: VirtAddr) -> Self {
    Self {
      fault_address, error_code, instr_address,
    }
  }
  pub fn instr_address(&self) -> VirtAddr {
    self.instr_address
  }
  pub fn fault_address(&self) -> VirtAddr {
    self.fault_address
  }
  pub fn page(&self) -> x86_64::structures::paging::Page {
    x86_64::structures::paging::Page::containing_address(self.fault_address)
  }
  pub fn error_code(&self) -> PageFaultErrorCode {
    self.error_code
  }
  pub fn assert_valid(&self) {
    assert!(!self.error_code.contains(PageFaultErrorCode::MALFORMED_TABLE), "Malformed Table");
  }
  pub fn caused_by_protection_violation(&self) -> bool {
    self.error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION)
  }
  pub fn caused_by_write(&self) -> bool {
    self.error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE)
  }
  pub fn caused_by_usermode(&self) -> bool {
    self.error_code.contains(PageFaultErrorCode::USER_MODE)
  }
  pub fn caused_by_instruction_fetch(&self) -> bool {
    self.error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH)
  }
  pub fn is_kstack(&self) -> bool {
    page_range!(KSTACK + 1).contains(&self.fault_address)
  }
  pub fn is_kheap(&self) -> bool {
    page_range!(KHEAP).contains(&self.fault_address)
  }
  pub fn is_ustack(&self) -> bool {
    page_range!(STACK + 1).contains(&self.fault_address)
  }
  pub fn is_udata(&self) -> bool {
    page_range!(DATA).contains(&self.fault_address)
  }
  pub fn is_ucode(&self) -> bool {
    page_range!(CODE).contains(&self.fault_address)
  }
}

impl core::fmt::Debug for PageFaultContext {
  fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(fmt, "{{ fault: {:?}, instr: {:?}, code: {:?} }}",
      self.fault_address(),
      self.instr_address(),
      self.error_code(),
    )
  }
}