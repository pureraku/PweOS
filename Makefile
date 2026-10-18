ASM = nasm
LD = ld.lld
CARGO = cargo
QEMU = qemu-system-i386
CLANG ?= clang
USER_ARCH_FLAGS = -march=i386 -mno-sse -mno-sse2 -mno-mmx -fno-vectorize -fno-slp-vectorize

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
DATA_IMAGE = $(BUILD)/data.img
STORAGE_IMAGE = $(BUILD)/storage.img
ISO_IMAGE = $(BUILD)/pweos.iso
ISO_STAGING = $(BUILD)/iso-staging

DATA_DRIVE = -drive file=$(DATA_IMAGE),format=raw,if=ide,index=0
STORAGE_DRIVE = -drive file=$(STORAGE_IMAGE),format=raw,if=ide,index=1

STORAGE_SIZE = 16M

ENTRY_OBJ = $(BUILD)/entry.asm.o
ISR_OBJ = $(BUILD)/isr.asm.o
USER_OBJ = $(BUILD)/user.asm.o
USER_CRT_OBJ = $(BUILD)/crt0.user.o
USER_RUNTIME_OBJ = $(BUILD)/pweos_libc.user.o
USER_STDIO_OBJ = $(BUILD)/pweos_stdio.user.o
USER_STDLIB_OBJ = $(BUILD)/pweos_stdlib.user.o
USER_ERRNO_OBJ = $(BUILD)/pweos_errno.user.o
USER_UNISTD_OBJ = $(BUILD)/pweos_unistd.user.o
USER_SETJMP_OBJ = $(BUILD)/setjmp.user.o
USER_TIME_OBJ = $(BUILD)/pweos_time.user.o
USER_MATH_OBJ = $(BUILD)/pweos_math.user.o
USER_PROGRAMS = $(shell cat examples/user_programs.txt)
USER_PROGRAM_ELFS = $(addprefix $(BUILD)/,$(addsuffix .elf,$(USER_PROGRAMS)))
TCC_OBJ = $(BUILD)/tcc.user.o
TCC_ELF = $(BUILD)/tcc.elf
TCC_SOURCES = $(wildcard third_party/tinycc/*.c third_party/tinycc/*.h third_party/tinycc/include/*.h examples/*.h examples/sys/*.h)
TCC_RUNTIME_OBJS = $(BUILD)/tcc_libtcc1.o $(BUILD)/tcc_atomic.o $(BUILD)/tcc_builtin.o $(BUILD)/tcc_alloca.o $(BUILD)/tcc_pic86.o
TCC_RUNTIME_OBJ = $(BUILD)/libtcc1.o
PWEOS_RUNTIME_OBJ = $(BUILD)/pweos_runtime.o
KERNEL_LIB = target/i686-pweos/release/libpweos.a

LDFLAGS = -m elf_i386 --gc-sections --oformat binary -T linker.ld

.PHONY: all iso storage run-iso run-iso-fullscreen run run-fullscreen run-curses run-serial run-headless clean check clippy

all: $(IMAGE) $(DATA_IMAGE)

iso: $(IMAGE) $(DATA_IMAGE)
	mkdir -p $(ISO_STAGING)
	cp $(IMAGE) $(ISO_STAGING)/pweos.img
	cp $(DATA_IMAGE) $(ISO_STAGING)/data.img
	@printf 'PweOS -- Bootable Floppy Disk Image\n=====================================\n\nTo run in QEMU (Linux/macOS):\n\n  qemu-system-i386 \\\n    -drive file=pweos.img,format=raw,if=floppy \\\n    -drive file=data.img,format=raw,if=ide,index=0 \\\n    -display default\n\nOn macOS (Homebrew QEMU):\n  qemu-system-i386 \\\n    -drive file=pweos.img,format=raw,if=floppy \\\n    -drive file=data.img,format=raw,if=ide,index=0 \\\n    -display cocoa,zoom-to-fit=on\n\nOn Windows (download from qemu.org):\n  qemu-system-i386.exe -drive file=pweos.img,format=raw,if=floppy -drive file=data.img,format=raw,if=ide,index=0\n\nIn VirtualBox:\n  1. Create new VM: Type=Other, Version=Other/Unknown (32-bit)\n  2. Skip disk creation\n  3. Settings > Storage > Add Floppy Controller > add pweos.img as floppy\n  4. Settings > Storage > Add IDE Controller  > add data.img  as hard disk\n  5. Boot the VM\n\nType '"'"'help'"'"' once booted for all commands.\nTry: video /videos/demo.pwevid   (demo video)\n     image /images/cat.rgb332    (image viewer)\n     tcc hello.c -o hello        (live C compiler!)\n' > $(ISO_STAGING)/README.txt
	xorriso -as mkisofs \
		-o $(ISO_IMAGE) \
		-V PWEOS \
		-J -R \
		-b pweos.img \
		-c boot.cat \
		-boot-load-size 2880 \
		$(ISO_STAGING)/
	@echo "ISO created: $(ISO_IMAGE)"

$(BUILD):
	mkdir -p $(BUILD)

$(STORAGE_IMAGE): | $(BUILD)
	@if [ ! -f "$@" ]; then \
		echo "Creating persistent storage image ($(STORAGE_SIZE))..."; \
		truncate -s $(STORAGE_SIZE) "$@"; \
		echo "Created $@"; \
	else \
		echo "Keeping existing persistent storage: $@"; \
	fi

storage: $(STORAGE_IMAGE)

$(BOOT_BIN): boot/boot.asm | $(BUILD)
	$(ASM) $< -f bin -o $@
	test "$$(wc -c < $(BOOT_BIN))" -eq 512

$(ENTRY_OBJ): src/arch/entry.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(ISR_OBJ): src/arch/isr.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(USER_OBJ): src/arch/user.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(BUILD)/%.user.o: examples/%.c $(wildcard examples/*.h) Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_CRT_OBJ): examples/crt0.S | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -c $< -o $@

$(USER_RUNTIME_OBJ): examples/pweos_libc.c examples/pweos_libc.h examples/string.h examples/pweos.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_STDIO_OBJ): examples/pweos_stdio.c examples/stdio.h examples/pweos_libc.h examples/pweos.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_STDLIB_OBJ): examples/pweos_stdlib.c examples/stdlib.h examples/ctype.h examples/pweos_libc.h examples/pweos.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_ERRNO_OBJ): examples/pweos_errno.c examples/errno.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_UNISTD_OBJ): examples/pweos_unistd.c examples/fcntl.h examples/unistd.h examples/stdio.h examples/stdlib.h examples/string.h examples/errno.h examples/pweos.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_SETJMP_OBJ): examples/setjmp.asm | $(BUILD)
	$(ASM) $< -f elf32 -o $@

$(USER_TIME_OBJ): examples/pweos_time.c examples/sys/time.h examples/time.h examples/pweos.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(USER_MATH_OBJ): examples/pweos_math.c examples/math.h examples/stdlib.h examples/ctype.h examples/errno.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -c $< -o $@

$(BUILD)/%.elf: $(USER_CRT_OBJ) $(BUILD)/%.user.o $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) $(TCC_RUNTIME_OBJ) examples/user.ld
	$(LD) -m elf_i386 -static -z max-page-size=0x1000 -T examples/user.ld $(USER_CRT_OBJ) $(BUILD)/$*.user.o $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) $(TCC_RUNTIME_OBJ) -o $@

$(PWEOS_RUNTIME_OBJ): $(USER_CRT_OBJ) $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) $(TCC_RUNTIME_OBJ) Makefile
	$(LD) -m elf_i386 -r $(USER_CRT_OBJ) $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) $(TCC_RUNTIME_OBJ) -o $@

$(BUILD)/tcc_libtcc1.o: third_party/tinycc/lib/libtcc1.c third_party/tinycc/config.h Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -fheinous-gnu-extensions -c $< -o $@

$(BUILD)/tcc_atomic.o: third_party/tinycc/lib/atomic.S | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -c $< -o $@

$(BUILD)/tcc_builtin.o: third_party/tinycc/lib/builtin.c Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -c $< -o $@

$(BUILD)/tcc_alloca.o: third_party/tinycc/lib/alloca.S | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -c $< -o $@

$(BUILD)/tcc_pic86.o: third_party/tinycc/lib/pic86.S | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 -c $< -o $@

$(TCC_RUNTIME_OBJ): $(TCC_RUNTIME_OBJS) Makefile
	$(LD) -m elf_i386 -r $(TCC_RUNTIME_OBJS) -o $@

$(TCC_OBJ): third_party/tinycc/tcc.c $(TCC_SOURCES) Makefile | $(BUILD)
	$(CLANG) --target=i386-unknown-none-elf -m32 $(USER_ARCH_FLAGS) -O2 -ffreestanding -fno-builtin -fno-pic -fno-pie -fno-stack-protector -Iexamples -Ithird_party/tinycc/include -Ithird_party/tinycc -c $< -o $@

$(TCC_ELF): $(USER_CRT_OBJ) $(TCC_OBJ) $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) $(TCC_RUNTIME_OBJ) examples/user.ld Makefile
	$(LD) -m elf_i386 -static -z max-page-size=0x1000 -T examples/user.ld $(USER_CRT_OBJ) $(TCC_OBJ) $(USER_RUNTIME_OBJ) $(USER_STDIO_OBJ) $(USER_STDLIB_OBJ) $(USER_ERRNO_OBJ) $(USER_UNISTD_OBJ) $(USER_SETJMP_OBJ) $(USER_TIME_OBJ) $(USER_MATH_OBJ) $(TCC_RUNTIME_OBJ) -o $@

$(KERNEL_LIB): $(wildcard src/*.rs) $(wildcard src/**/*.rs) $(wildcard assets/*.rgb332) $(wildcard assets/*.pwevid) $(USER_PROGRAM_ELFS) examples/user_programs.txt $(TCC_ELF) $(TCC_RUNTIME_OBJ) $(PWEOS_RUNTIME_OBJ) build.rs Cargo.toml targets/i686-pweos.json
	$(CARGO) build --release

$(KERNEL_BIN): $(ENTRY_OBJ) $(ISR_OBJ) $(USER_OBJ) $(KERNEL_LIB)
	$(LD) $(LDFLAGS) $^ -o $@
	test "$$(wc -c < $@)" -le 458752

$(IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(IMAGE)
	truncate -s 1440K $(IMAGE)

$(DATA_IMAGE): $(wildcard assets/*.pwevid) $(USER_PROGRAM_ELFS) examples/user_programs.txt $(TCC_ELF) tools/pack_data_disk.py | $(BUILD)
	python3 tools/pack_data_disk.py assets examples/user_programs.txt $(BUILD) $(TCC_ELF) $(DATA_IMAGE)

run: $(IMAGE) $(DATA_IMAGE) $(STORAGE_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(DATA_DRIVE) $(STORAGE_DRIVE) $(QEMU_DISPLAY)

run-fullscreen: $(IMAGE) $(DATA_IMAGE) $(STORAGE_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(DATA_DRIVE) $(STORAGE_DRIVE) $(QEMU_FULLSCREEN)

run-curses: $(IMAGE) $(DATA_IMAGE) $(STORAGE_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(DATA_DRIVE) $(STORAGE_DRIVE) -display curses

run-serial: $(IMAGE) $(DATA_IMAGE) $(STORAGE_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(DATA_DRIVE) $(STORAGE_DRIVE) $(QEMU_DISPLAY) -serial stdio

run-headless: $(IMAGE) $(DATA_IMAGE) $(STORAGE_IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy $(DATA_DRIVE) $(STORAGE_DRIVE) -display none -serial stdio

run-iso: iso $(STORAGE_IMAGE)
	$(QEMU) -cdrom $(ISO_IMAGE) -boot d $(STORAGE_DRIVE) -vga std $(QEMU_DISPLAY)

run-iso-fullscreen: iso $(STORAGE_IMAGE)
	$(QEMU) -cdrom $(ISO_IMAGE) -boot d $(STORAGE_DRIVE) -vga std $(QEMU_FULLSCREEN)

check:
	$(CARGO) check

clippy:
	$(CARGO) clippy

clean:
	@if [ -f "$(STORAGE_IMAGE)" ]; then \
		cp "$(STORAGE_IMAGE)" "$(STORAGE_IMAGE).backup"; \
	fi
	rm -rf $(BUILD) target
	@if [ -f "$(STORAGE_IMAGE).backup" ]; then \
		mkdir -p $(BUILD); \
		mv "$(STORAGE_IMAGE).backup" "$(STORAGE_IMAGE)"; \
	fi
