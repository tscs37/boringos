.PHONY: all clean kernel release rustup pid0_build

KERNEL_TARGET = x86_64-boringoscore
BIN_TARGET = x86_64-boringosbase
CRATE = boringos
QEMU_MEMORY = 512
QEMU_PLATFORM = system-x86_64
KERNEL_BUILD_MODE = debug
BOOTIMG_FILE = target/$(KERNEL_TARGET)/$(KERNEL_BUILD_MODE)/bootimage-$(CRATE).bin
BIN_FILE = target/$(KERNEL_TARGET)/debug/$(CRATE)
QEMU_OPTIONS = -net none -m $(QEMU_MEMORY) \
	-vga cirrus -cpu Broadwell \
	-drive if=ide,format=raw,file=$(BOOTIMG_FILE) \
	-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
	-serial mon:stdio --no-reboot

build: bootimage

all: bootimage qemu

release: qemu_release

rustup: .rustup
	@rustup toolchain add nightly-2019-01-01
	@rustup override add nightly-2019-01-01
	@rustup component add rust-src
	@rustup component add rls-preview rust-analysis
	@cargo install cargo-xbuild --force
	@cargo install bootimage --version "^0.5.0" --force

ln_targets: pid0/$(BIN_TARGET).json

pid0/$(BIN_TARGET).json: $(BIN_TARGET).json
	@echo "Copy new target configuration for BIN_TARGET"
	@cp $(PWD)/$(BIN_TARGET).json $(PWD)/pid0/$(BIN_TARGET).json

clean: clean_kernel clean_pid0

clean_kernel:
	@echo "Cleaning Kernel"
	@rm -r target/ || exit 0

clean_pid0:
	@echo "Cleaning PID0"
	@rm -r pid0/target/ || exit 0

bootimage: initramdata/pid0 initramdata/initramfs.bin kernel

kernel:
	@echo "Building Kernel image"
ifeq ($(KERNEL_BUILD_MODE),debug)
	@bootimage build --target $(KERNEL_TARGET).json
else
	@bootimage build --$(KERNEL_BUILD_MODE) --target $(KERNEL_TARGET).json
endif

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