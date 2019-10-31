# BoringOS

BoringOS, BOS for short, is a project to design a nanokernel underpinning a modern operating system with the maximum degrees of freedom. It's core principles update on UNIX-oid development and aim to provide a flexible, introspectable system.

## Building and Running BOS

BOS is a rust-based operating system and requires the rust compiler, the rustup toolkit and QEMU to be installed for development.

Running `make rustup` will setup the toolchain for you.

Running `make` will build the initramfs, PID0 and kernel, assemble the bootimage and launch QEMU.

The script `look_adr.sh` can be used to translate an address into a position in the BOS source code.

## Kernel

The BOS Kernel is a nanokernel, all tasks other than bootup, process and memory management are delegated to userspace. To simplify the kernel further, processes run in ring 0 and are responsible for running in ring 3 themselves, if that is desired.

The scheduler operates on a voluntary timesharing principle and allows specifying a scheduler process that will yield to other processes as needed and appropriate. By itself the kernel can only yield to a specific process, defaulting to the scheduler if nothing is specified.

Processes are assigned a 128bit random process ID the identifies the process. This ID is referred to as "task handle". The special handle 0 has a context-sensitive meaning, the kernel interpretes this as the scheduler process but it may have other meanings in higher level APIs.

Memory is managed on demand and automatically, however, processes do have a page limit, the number of pages assigned to a process (in 4KiB pages). Heap must be touched sequentially, it is not legal to access heap addresses beyond the highest address last access, rounded up to the next page boundary.

When talking about processes, generally this refers to tasks. A task is the fundamental building block of multithreading in BOS. Tasks can refer to a singular process or to threads of a program. A task may share memory and code with another task, like a thread, or it might not be related at all. Itself, a task is a lightweight abstraction that can be arbitrarily distances from it's parent task.

When booting, the kernel creates too processes; the null task and the PID0 task. The nulltask crashes the kernel if executed and is assigned as scheduler on boot. The PID0 task is hardcoded into the kernel binary and is responsible for unpacking the initramfs and setting up a proper scheduler.

### PID0

PID0 is the first proper process spawned by the kernel and is responsible for loading the initramfs and setting up the core system itself.

The initramfs is hardcoded into the kernel, it is not directly comparable to the initramfs of Linux. The purpose of this image is to provide all components and drivers to operate the hardware of a system sufficiently to bootstrap the actual disk images (like a traditional initramfs).