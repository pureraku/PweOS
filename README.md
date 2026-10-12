# PweOS

PweOS is a minimal, educational 32-bit x86 operating system featuring a custom BIOS bootloader, freestanding C kernel, interrupt-driven device drivers, and an interactive shell.

```
================================================================================
                      Welcome to PweOS (32-bit x86)                             
================================================================================

[+] CPU: Intel 80386+ in 32-bit Protected Mode (IF=1 interrupts active)
[+] Memory: Flat model active (base: 0x0, limit: 4GB)
[+] IDT: 256 descriptor gates loaded, PIC remapped to vectors 0x20..0x2F
[+] PIT: Timer running at 100 Hz (IRQ0 active)
[+] Keyboard: PS/2 controller driver online (IRQ1 active)
[+] Console: VGA 80x25 text buffer at 0xb8000
[+] Serial: COM1 active at port 0x3f8 (38400 baud, 8N1)

Type 'help' to see available commands.

pweos> help

PweOS Builtin Commands:
----------------------------------------------------
  help       - Display list of available commands
  clear      - Clear screen and reset cursor
  echo       - Print arguments to the console
  version    - Display PweOS kernel version and arch
  uname      - Display operating system information
  uptime     - Display system uptime and timer ticks
  calc       - Evaluate arithmetic expression: a op b
  color      - Set terminal color: color <fg> [bg]
  sleep      - Delay execution for N seconds
  peek       - Read byte from memory: peek <hex_addr>
  reboot     - Reboot the system
  panic      - Trigger kernel panic (exception test)
  about      - Information about PweOS system
----------------------------------------------------
pweos>
```

---

## Architecture Overview

PweOS is built from the ground up without third-party runtimes:

```
+-------------------------------------------------------------+
|                     PweOS Interactive Shell                 |
|       (Line editing, command parsing, builtin utilities)    |
+-------------------------------------------------------------+
|                      Custom Freestanding C Libc              |
|        (printf, snprintf, string, ctype, memory utilities)  |
+-------------------------------------------------------------+
|                       Device Drivers                        |
|   VGA 80x25 Text Mode | PS/2 Keyboard | PIT 8253 | UART COM1 |
+-------------------------------------------------------------+
|                 x86 Interrupt Subsystem                     |
|  256-Entry IDT | 8259 PIC Remapper | ISR Stubs & Panic Dump  |
+-------------------------------------------------------------+
|                    32-bit Protected Mode                    |
|   GDT (Flat 4GB Code & Data) | A20 Gate | Stack at 0x90000   |
+-------------------------------------------------------------+
|              Stage 1 BIOS Boot Sector (512 Bytes)           |
|      Loads 32KB kernel from floppy disk via BIOS INT 13h    |
+-------------------------------------------------------------+
```

### 1. Bootloader (`boot/boot.asm`)
- **Stage 1 MBR Boot Sector (512 bytes)** executed by BIOS at `0x7C00`.
- Performs **LBA to CHS conversion** to load 64 sectors (32 KB) of the kernel into physical memory at `0x10000` (`0x1000:0x0000`).
- Enables the **A20 Address Line** via Fast A20 gate.
- Loads a 32-bit **Global Descriptor Table (GDT)** defining flat 4GB code and data segments.
- Transitions the processor from 16-bit Real Mode to **32-bit Protected Mode** (`CR0.PE = 1`) and jumps to kernel entry.

### 2. Kernel Entry & Linker (`kernel/arch/entry.asm`, `kernel/linker.ld`)
- Sets up 32-bit data segment registers (`DS`, `ES`, `FS`, `GS`, `SS`).
- Initializes a dedicated stack safely below video memory (`0x90000`).
- Transfers control to `kmain()`.

### 3. VGA Text Console & Serial Driver (`kernel/drivers/vga.c`, `kernel/drivers/vga.h`)
- Memory-mapped text buffer at physical address `0xB8000` (80 columns x 25 rows).
- Supports 16 foreground and 16 background colors.
- Handles `\n`, `\r`, `\t`, backspace (`\b`), and screen scrolling.
- Updates the hardware blinking text cursor via CRT controller I/O ports `0x3D4` and `0x3D5`.
- Simultaneously mirrors all output to **COM1 serial port (`0x3F8`)** at 38400 baud.

### 4. Interrupts & Exceptions (`kernel/arch/`)
- **IDT (`idt.c`, `idt.h`)**: Configures 256 interrupt gates.
- **PIC 8259 (`pic.c`, `pic.h`)**: Remaps Master PIC IRQs to vectors `0x20..0x27` and Slave PIC IRQs to vectors `0x28..0x2F` to prevent conflicts with Intel CPU exceptions.
- **ISRs (`isr.asm`, `isr.c`)**: Dedicated assembly stubs for CPU exceptions 0-31 that save CPU registers (`registers_t`) and invoke diagnostic panic handlers displaying exception names, error codes, and full register dumps.

### 5. Hardware Drivers (`kernel/drivers/`)
- **PIT 8253 Timer (`timer.c`, `timer.h`)**: Channel 0 timer running at 100 Hz on IRQ0, tracking system ticks, elapsed uptime, and supporting `timer_sleep()`.
- **PS/2 Keyboard (`keyboard.c`, `keyboard.h`)**: Decodes Scancode Set 1 on IRQ1, manages Shift and Caps Lock states, buffers keystrokes in a circular ring buffer, and provides interactive line editing (`keyboard_readline`). Also supports headless serial input from COM1.

### 6. Freestanding C Standard Library (`kernel/lib/`)
- Minimal custom C runtime without external dependencies.
- Memory & string manipulation: `strlen`, `strcmp`, `strncmp`, `strcpy`, `strncpy`, `memset`, `memcpy`, `memmove`, `atoi`, `itoa`, `utoa`.
- Formatted printing: `printf`, `snprintf`, `vsnprintf` supporting `%s`, `%c`, `%d`, `%u`, `%x`, `%X`, `%p`, and field width/zero-padding.

### 7. Interactive Shell (`kernel/shell/`)
- Clean prompt (`pweos> `) with real-time backspace character deletion and newline submission.
- Zero-allocation command line tokenization into `argc` and `argv`.

---

## Builtin Shell Commands

| Command | Syntax | Description |
|---|---|---|
| `help` | `help` | Lists all available commands and their descriptions |
| `clear` | `clear` | Clears the VGA text screen and resets cursor position |
| `echo` | `echo [text...]` | Prints the arguments to standard output |
| `version` / `uname` | `version` | Displays OS name, version, and target architecture |
| `uptime` | `uptime` | Displays uptime in minutes and seconds, along with timer ticks |
| `calc` | `calc <a> <op> <b>` | Performs arithmetic: `+`, `-`, `*`, `/`, `%` with div-by-zero protection |
| `color` | `color <fg> [bg]` | Changes console foreground and optional background color |
| `sleep` | `sleep <seconds>` | Delays execution for N seconds using timer interrupts |
| `peek` | `peek <hex_addr>` | Reads and displays the byte stored at physical memory address |
| `reboot` | `reboot` | Restarts the machine via 8042 keyboard controller reset line |
| `panic` | `panic` | Triggers a division-by-zero exception to demonstrate the crash handler |
| `about` | `about` | Displays system architecture and component specifications |

---

## Repository Structure

```
PweOS/
├── Makefile                 # Build system & QEMU targets
├── README.md                # Documentation & Architecture overview
├── boot/
│   └── boot.asm             # Stage 1 MBR Bootloader (16-bit to 32-bit switch)
└── kernel/
    ├── kernel.c             # Kernel initialization (kmain)
    ├── linker.ld            # Linker script (entry at 0x10000)
    ├── arch/
    │   ├── entry.asm        # 32-bit kernel startup & C calling convention
    │   ├── idt.c / idt.h    # Interrupt Descriptor Table management
    │   ├── io.h             # Inline assembly for x86 port I/O (inb, outb)
    │   ├── isr.asm          # Low-level interrupt service routine stubs
    │   ├── isr.c / isr.h    # Exception and IRQ dispatchers
    │   └── pic.c / pic.h    # 8259 Programmable Interrupt Controller
    ├── drivers/
    │   ├── keyboard.c / .h  # PS/2 keyboard & serial input driver
    │   ├── timer.c / .h     # 8253 PIT timer (100 Hz, uptime, sleep)
    │   └── vga.c / .h       # VGA 80x25 text mode & COM1 UART serial
    ├── lib/
    │   ├── printf.c / .h    # Formatted printk/printf implementation
    │   └── string.c / .h    # Memory and string manipulation functions
    └── shell/
        ├── shell.c / .h     # Interactive shell REPL and builtins
```

---

## Prerequisites

To build and run PweOS, you need:
- **NASM** (Netwide Assembler)
- **Clang** (with `i686-unknown-none-elf` target support)
- **LLD** (`ld.lld` linker)
- **QEMU** (`qemu-system-i386`)

On macOS (via Homebrew):
```bash
brew install nasm lld qemu
```

*(Note: Apple Clang included with Xcode Command Line Tools natively supports `--target=i686-unknown-none-elf`)*

---

## Building and Running

### Build the OS image
```bash
make
```
This compiles the bootloader and kernel, and creates a bootable floppy disk image at `build/pweos.img`.

### Run in QEMU (GUI window)
```bash
make run
```

### Run in QEMU with Serial Port Output
```bash
make run-serial
```
Outputs both to the graphical VGA window and mirrors all text to your host terminal via COM1.

### Run Headless (CLI only)
```bash
make run-headless
```
Runs completely in your terminal without opening a graphical window. You can interact directly with the shell using your terminal keyboard!

### Clean build artifacts
```bash
make clean
```
