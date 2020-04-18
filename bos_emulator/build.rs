
use std::process::Command;
use std::fs::{copy, create_dir};
fn main() {
  const TESTPROC_DIR: &str = "./bosemu-testproc";
  const TESTPROC_OUT: &str = "./bosemu-testproc/target/wasm32-unknown-unknown/debug/bosemu_testproc.wasm";
  Command::new("cargo").arg("build").current_dir(TESTPROC_DIR).output().unwrap();
  Command::
    new("cargo")
    .arg("build")
    .arg("--target=wasm32-unknown-unknown")
    .current_dir(TESTPROC_DIR)
    .output().unwrap();
  if !std::path::Path::new("./res/").exists() {
    create_dir("./res/").unwrap();
  }
  copy(
    TESTPROC_OUT,
    "./res/bosemu_testproc.wasm"
  ).unwrap();
}
