ASM = nasm
LD = ld.lld
CARGO = cargo
QEMU = qemu-system-i386
CLANG ?= clang

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    QEMU_DISPLAY ?= -display cocoa,zoom-to-fit=on
    QEMU_FULLSCREEN ?= -display cocoa,zoom-to-fit=on,full-screen=on
else
    QEMU_DISPLAY ?= -display default
    QEMU_FULLSCREEN ?= -full-screen
endif

BUILD = build
BOOT_BIN = $(BUILD)/boot.bin
KERNEL_BIN = $(BUILD)/kernel.bin
IMAGE = $(BUILD)/pweos.img
VIDEO_IMAGE = $(BUILD)/videos.img
VIDEO_DRIVE = -drive file=$(VIDEO_IMAGE),format=raw,if=ide,index=0

ENTRY_OBJ = $(BUILD)/entry.asm.o
ISR_OBJ = $(BUILD)/isr.asm.o
USER_OBJ = $(BUILD)/user.asm.o
USER_CRT_OBJ = $(BUILD)/crt0.user.o
USER_HELLO_OBJ = $(BUILD)/hello.user.o
USER_RUNTIME_OBJ = $(BUILD)/pweos_libc.user.o
USER_STDIO_OBJ = $(BUILD)/pweos_stdio.user.o
USER_STDLIB_OBJ = $(BUILD)/pweos_stdlib.user.o
USER_ERRNO_OBJ = $(BUILD)/pweos_errno.user.o
USER_UNISTD_OBJ = $(BUILD)/pweos_unistd.user.o
USER_SETJMP_OBJ = $(BUILD)/setjmp.user.o
USER_TIME_OBJ = $(BUILD)/pweos_time.user.o
USER_MATH_OBJ = $(BUILD)/pweos_math.user.o
USER_HELLO_ELF = $(BUILD)/hello.elf
KERNEL_LIB = target/i686-pweos/release/libpweos.a

LDFLAGS = -m elf_i386 --gc-sections --oformat binary -T linker.ld

.PHONY: all run run-fullscreen run-curses run-serial run-headless clean check clippy

all: $(IMAGE) $(VIDEO_IMAGE)

$(BUILD):
	mkdir -p $(BUILD)

$(BOOT_BIN): boot/boot.asm | $(BUILD)
	$(ASM) $< -f bin -o $@
	test "$$(wc -c < $(BOOT_BIN))" -eq 512

$(ENTRY_OBJ): src/arch/entry.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(ISR_OBJ): src/arch/isr.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(USER_OBJ): src/arch/user.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(USER_HELLO_OBJ): examples/hello.c examples/stdio.h examples/pweos.h examples/pweos_libc.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_CRT_OBJ): examples/crt0.S | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -c $< -o $@

$(USER_RUNTIME_OBJ): examples/pweos_libc.c examples/pweos_libc.h examples/string.h examples/pweos.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_STDIO_OBJ): examples/pweos_stdio.c examples/stdio.h examples/pweos_libc.h examples/pweos.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_STDLIB_OBJ): examples/pweos_stdlib.c examples/stdlib.h examples/ctype.h examples/pweos_libc.h examples/pweos.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_ERRNO_OBJ): examples/pweos_errno.c examples/errno.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_UNISTD_OBJ): examples/pweos_unistd.c examples/fcntl.h examples/unistd.h examples/pweos.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_SETJMP_OBJ): examples/setjmp.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(USER_TIME_OBJ): examples/pweos_time.c examples/sys/time.h examples/time.h examples/pweos.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_MATH_OBJ): examples/pweos_math.c examples/math.h examples/stdlib.h examples/ctype.h examples/errno.h | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_HELLO_ELF): $(USER_CRT_OBJ) $(USER_HELLO_OBJ) $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) examples/user.ld
	$(LD) -m elf_i386 -static -z max-page-size=0x1000 -T examples/user.ld $(USER_CRT_OBJ) $(USER_HELLO_OBJ) $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) -o $@

$(KERNEL_LIB): $(wildcard src/*.rs) $(wildcard src/**/*.rs) $(wildcard assets/*.rgb332) $(wildcard assets/*.pwevid) $(USER_HELLO_ELF) build.rs Cargo.toml targets/i686-pweos.json
	$(CARGO) build --release

$(KERNEL_BIN): $(ENTRY_OBJ) $(ISR_OBJ) $(USER_OBJ) $(KERNEL_LIB)
	$(LD) $(LDFLAGS) $^ -o $@
	test "$$(wc -c < $@)" -le 458752

$(IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(IMAGE)
	truncate -s 1440K $(IMAGE)

$(VIDEO_IMAGE): $(wildcard assets/*.pwevid) tools/pack_videos.py | $(BUILD)
	python3 tools/pack_videos.py assets $(VIDEO_IMAGE)

run: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) $(QEMU_DISPLAY)

run-fullscreen: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) $(QEMU_FULLSCREEN)

run-curses: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) -display curses

run-serial: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) $(QEMU_DISPLAY) -serial stdio

run-headless: $(IMAGE) $(VIDEO_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(VIDEO_DRIVE) -display none -serial stdio

check:
	$(CARGO) check

clippy:
	$(CARGO) clippy

clean:
	rm -rf $(BUILD) target
