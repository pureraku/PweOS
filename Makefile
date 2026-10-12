ASM = nasm
QEMU = qemu-system-i386

SRC = src/main.asm
BUILD = build
BOOT_BIN = $(BUILD)/main.bin
IMAGE = $(BUILD)/pweos.img

.PHONY: all run clean

all: $(IMAGE)

$(BUILD):
	mkdir -p $(BUILD)

$(BOOT_BIN): $(SRC) | $(BUILD)
	$(ASM) $(SRC) -f bin -o $(BOOT_BIN)

$(IMAGE): $(BOOT_BIN)
	test "$$(wc -c < $(BOOT_BIN))" -eq 512
	cp $(BOOT_BIN) $(IMAGE)
	truncate -s 1440K $(IMAGE)

run: $(IMAGE)
	$(QEMU) -drive file=$(IMAGE),format=raw,if=floppy

clean:
	rm -rf $(BUILD)
