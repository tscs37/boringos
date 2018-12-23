.PHONY: all clean kernel release rustup pid0_build

KERNEL_TARGET = x86_64-boringoscore
BIN_TARGET = x86_64-boringosbase
CRATE = boringos
QEMU_MEMORY = 512
QEMU_PLATFORM = system-x86_64
BOOTIMG_FILE = target/$(KERNEL_TARGET)/debug/bootimage-$(CRATE).bin
BIN_FILE = target/$(KERNEL_TARGET)/debug/$(CRATE)
QEMU_OPTIONS = -net none -m $(QEMU_MEMORY) \
	-vga cirrus --enable-kvm --cpu host \
	-drive if=ide,format=raw,file=$(BOOTIMG_FILE) \
	-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
	-serial mon:stdio --no-reboot -d cpu_reset,int

all: kernel bootimage qemu

release: qemu_release

rustup: .rustup
	@rustup toolchain add nightly-2018-12-20
	@rustup override add nightly-2018-12-20
	@rustup component add rust-src
	@rustup component add rls-preview rust-analysis
	@cargo install cargo-xbuild --force
	@cargo install bootimage --version "^0.5.0" --force

ln_targets: pid0/$(BIN_TARGET).json

pid0/$(BIN_TARGET).json:
	@cp $(PWD)/$(BIN_TARGET).json $(PWD)/pid0/$(BIN_TARGET).json

clean:
	rm -r target/

bootimage: initramdata/pid0 initramdata/initramfs.bin
	@echo "Building Kernel image"
	@bootimage build --target $(KERNEL_TARGET).json

initramdata/pid0: ln_targets pid0_build
	@cp pid0/target/$(BIN_TARGET)/debug/pid0 initramdata/pid0

pid0_build: ln_targets
	@echo "Building PID0 binary"
	@cd pid0 && cargo xbuild --target $(BIN_TARGET).json

initramdata/initramfs.bin:
	@echo "Building InitRAMFS image"
	@touch initramdata/initramfs.bin

qemu: bootimage
	@qemu-$(QEMU_PLATFORM) $(QEMU_OPTIONS) || exit 0