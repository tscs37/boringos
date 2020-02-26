mod pagemap_ng;

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    info!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    use crate::bindriver::cpu::qemu::*;
    exit_qemu(QemuExitCode::Success);
}