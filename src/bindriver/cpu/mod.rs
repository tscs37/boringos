pub mod idt;
pub mod gdt;

use ::raw_cpuid::{CpuId, FeatureInfo};
use ::core::fmt::{Result, Write};

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