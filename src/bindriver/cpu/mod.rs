pub mod gdt;
pub mod idt;
pub mod pic;
use raw_cpuid::{CpuId, FeatureInfo};

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
