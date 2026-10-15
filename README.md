# PweOS

PweOS is a minimal, educational 32-bit x86 operating system featuring a custom BIOS bootloader, freestanding Rust kernel (`no_std`, `alloc`), dynamic heap memory allocator implementing Rust's `GlobalAlloc`, interrupt-driven device drivers, in-memory virtual filesystem, and an interactive command shell.

```
================================================================================
                      Welcome to PweOS (32-bit x86)                             
================================================================================

[+] CPU: Intel 80386+ in 32-bit Protected Mode (IF=1 interrupts active)
[+] Memory: Flat model active (base: 0x0, limit: 4GB)
[+] Heap: Dynamic allocator initialized at 0x100000 (size: 2048 KB)
[+] RAMFS: In-memory filesystem initialized (3 default files mounted)
[+] IDT: 256 descriptor gates loaded, PIC remapped to vectors 0x20..0x2F
[+] PIT: Timer running at 100 Hz (IRQ0 active)
[+] Keyboard: PS/2 controller driver online (IRQ1 active)
[+] Console: VGA 80x25 text buffer at 0xb8000
[+] Serial: COM1 active at port 0x3f8 (38400 baud, 8N1)

Type 'help' to see available commands.

pweos> help

PweOS Builtin Commands:
-----------------------:3---------------------------
  help       - Display list of available commands
  clear      - Clear screen and reset cursor
  echo       - Print arguments to the console
  version    - Display PweOS kernel version and arch
  uname      - Display operating system information
  uptime     - Display system uptime and timer ticks
  date       - Display real-time clock (CMOS RTC) date
  cpuinfo    - Display CPU hardware details via CPUID
  meminfo    - Display kernel heap memory statistics
  history    - Display command history buffer
  ls         - List files and directories in in-memory RAM filesystem
  mkdir      - Create a directory: mkdir <name>
  rmdir      - Remove an empty directory: rmdir <path>
  nuke       - Recursively remove a file or directory tree: nuke <path>
  cd         - Change directory: cd <path>
  pwd        - Print the current working directory
  cat        - Display file contents: cat <filename>
  grep       - Filter piped lines: grep <text>
  touch      - Create empty file: touch <filename>
  write      - Write one line: write <filename> <text>
  edit       - Edit multiline files: edit <path> (Escape saves)
  rm         - Delete file: rm <filename>
  calc       - Evaluate arithmetic expression: a op b
  color      - Set terminal color: color <fg> [bg]
  sleep      - Delay execution for N seconds
  peek       - Read byte from memory: peek <hex_addr>
  reboot     - Reboot the system
  panic      - Trigger kernel panic (exception test)
  about      - Information about PweOS system
-----------------------:3----------------------------
pweos>
```

---

## Architecture Overview

PweOS is built from the ground up in freestanding Rust without external operating system dependencies:

```
+-------------------------------------------------------------+
|                     PweOS Interactive Shell                 |
|       (Line editing, command history, arrow navigation)     |
+-------------------------------------------------------------+
|               RAMFS In-Memory Virtual Filesystem            |
|       (Dynamically allocated files, cat, touch, write, rm)   |
+-------------------------------------------------------------+
|                  Dynamic Heap Memory Allocator              |
|   (kmalloc/kfree, block coalescing, GlobalAlloc for alloc)  |
+-------------------------------------------------------------+
|             Freestanding Rust Runtime (no_std)              |
|        (core::fmt, alloc::vec::Vec, alloc::string::String)  |
+-------------------------------------------------------------+
|                       Device Drivers                        |
|  VGA 80x25 Text | PS/2 Keyboard | PIT 8253 | CMOS RTC | COM1|
+-------------------------------------------------------------+
|                 x86 Interrupt Subsystem                     |
|  256-Entry IDT | 8259 PIC Remapper | ISR Stubs & Panic Dump  |
+-------------------------------------------------------------+
|                    32-bit Protected Mode                    |
|   GDT (Flat 4GB Code & Data) | A20 Gate | Stack at 0x90000   |
+-------------------------------------------------------------+
|              Stage 1 BIOS Boot Sector (512 Bytes)           |
|      Loads kernel sectors from floppy disk via BIOS INT 13h |
+-------------------------------------------------------------+
```

### 1. Bootloader (`boot/boot.asm`)
- **Stage 1 MBR Boot Sector (512 bytes)** executed by BIOS at `0x7C00`.
- Performs **LBA to CHS conversion** to load kernel sectors into physical memory starting at `0x10000` (`0x1000:0x0000`), advancing `ES` segment by `0x20` per sector to support kernels larger than 64 KB.
- Enables the **A20 Address Line** via Fast A20 gate.
- Loads a 32-bit **Global Descriptor Table (GDT)** defining flat 4GB code and data segments.
- Transitions the processor from 16-bit Real Mode to **32-bit Protected Mode** (`CR0.PE = 1`) and jumps to kernel entry.

### 2. Kernel Entry & Linker (`src/arch/entry.asm`, `linker.ld`)
- Sets up 32-bit data segment registers (`DS`, `ES`, `FS`, `GS`, `SS`).
- Initializes a dedicated stack safely below video memory (`0x90000`).
- Transfers control to Rust kernel entry point `kmain()`.

### 3. Dynamic Heap Memory Manager (`src/mm/heap.rs`)
- Manages a 2 MB dynamic heap starting at `0x100000` (1 MB mark).
- Linked-list block allocator with 8-byte alignment, block splitting, and automatic boundary coalescing on free.
- Implements Rust's `core::alloc::GlobalAlloc` trait, enabling full use of the standard `alloc` crate (`String`, `Vec`, `Box`, `format!`).
- Provides live memory diagnostics via `meminfo`.

### 4. RAMFS Virtual Filesystem (`src/fs/ramfs.rs`)
- Dynamic in-memory virtual filesystem backed by the heap.
- Supports file creation, resizing, reading, appending, and deletion.
- Prepopulated with default virtual files: `motd`, `os-release`, and `readme.txt`.

### 5. VGA Text Console & Serial Driver (`src/drivers/vga.rs`, `src/drivers/serial.rs`)
- Memory-mapped text buffer at physical address `0xB8000` (80 columns x 25 rows).
- Supports 16 foreground and 16 background colors.
- Handles `\n`, `\r`, `\t`, backspace (`\b`), and screen scrolling.
- Updates the hardware blinking text cursor via CRT controller I/O ports `0x3D4` and `0x3D5`.
- Implements `core::fmt::Write` with `print!` and `println!` macros.
- Simultaneously mirrors all output to **COM1 serial port (`0x3F8`)** at 38400 baud.

### 6. Interrupts & Exceptions (`src/arch/`)
- **IDT (`idt.rs`)**: Configures 256 interrupt gates.
- **PIC 8259 (`pic.rs`)**: Remaps Master PIC IRQs to vectors `0x20..0x27` and Slave PIC IRQs to vectors `0x28..0x2F` to prevent conflicts with Intel CPU exceptions.
- **ISRs (`isr.asm`, `isr.rs`)**: Dedicated assembly stubs for CPU exceptions 0-31 that save CPU registers (`Registers`) and invoke diagnostic panic handlers displaying exception names, error codes, and full register dumps.

### 7. Hardware Drivers (`src/drivers/`)
- **PIT 8253 Timer (`timer.rs`)**: Channel 0 timer running at 100 Hz on IRQ0, tracking system ticks, elapsed uptime, and supporting `timer_sleep()`.
- **PS/2 Keyboard (`keyboard.rs`)**: Decodes Scancode Set 1 on IRQ1 with extended key prefix `0xE0` support for arrow keys (Up, Down, Left, Right). Handles Shift and Caps Lock, buffers keystrokes in a thread-safe ring buffer, and supports headless ANSI serial input from COM1.
- **CMOS Real-Time Clock (`rtc.rs`)**: Reads hardware RTC registers with BCD conversion, 12h/24h detection, and status checking for accurate wall-clock timestamps.
- **CPUID Detection (`cpu.rs`)**: Queries CPU vendor string, processor model/brand string, family/stepping, and hardware capability flags.

### 8. Interactive Shell (`src/shell/`)
- Colored prompt (`pweos> `) with backspace character deletion.
- **Command History**: Remembers up to 16 previous commands. Press **Up Arrow** and **Down Arrow** to cycle through history!
- Interactive command interpreter with line editing and 22 builtin commands.

---

## Builtin Shell Commands

| Command | Syntax | Description |
|---|---|---|
| `help` | `help` | Lists all available commands and their descriptions |
| `clear` | `clear` | Clears the VGA text screen and resets cursor position |
| `echo` | `echo [text...]` | Prints the arguments to standard output |
| `version` / `uname` | `version` | Displays OS name, version, and target architecture |
| `uptime` | `uptime` | Displays uptime in minutes and seconds, along with timer ticks |
| `date` | `date` | Reads and displays current CMOS Real-Time Clock date and time |
| `cpuinfo` | `cpuinfo` | Queries CPUID to display vendor, brand string, and feature flags |
| `meminfo` | `meminfo` | Displays total heap capacity, used bytes, free bytes, and blocks |
| `history` | `history` | Displays previous commands stored in the shell history buffer |
| `ls` | `ls [path]` | Lists the current directory or a selected path; directory names end in `/` |
| `mkdir` | `mkdir <path>` | Creates a directory under an existing parent directory |
| `rmdir` | `rmdir <path>` | Removes an empty directory |
| `nuke` | `nuke <path>` | Recursively removes a file or directory and its contents; protects the filesystem root and current working directory |
| `cd` | `cd <path>` | Changes the working directory; supports `..` and absolute paths |
| `pwd` | `pwd` | Prints the current working directory |
| `cat` | `cat <filename>` | Displays the contents of a file |
| `grep` | `grep <text>` | Prints lines from piped input that contain the text |
| `touch` | `touch <filename>` | Creates an empty file in RAMFS |
| `write` | `write <filename> <text>` | Replaces a file with one line of text |
| `edit` | `edit <path>` | Opens a multiline editor; arrows move the cursor, Enter inserts a newline, Backspace/Delete erase, and Escape saves |
| `rm` | `rm <filename>` | Deletes a file from RAMFS and reclaims memory |
| `calc` | `calc <a> <op> <b>` | Performs arithmetic: `+`, `-`, `*`, `/`, `%` with div-by-zero protection |
| `color` | `color <fg> [bg]` | Changes console foreground and optional background color |
| `sleep` | `sleep <seconds>` | Delays execution for N seconds using timer interrupts |
| `peek` | `peek <hex_addr>` | Reads and displays the byte stored at physical memory address |
| `reboot` | `reboot` | Restarts the machine via 8042 keyboard controller reset line |
| `panic` | `panic` | Triggers a division-by-zero exception to demonstrate the crash handler |
| `about` | `about` | Displays system architecture and component specifications |

Commands can be connected with `|` to pass output to the next command. For example, `cat notes.txt | grep TODO` filters a file, and `cat notes.txt | write copy.txt` writes its contents to another file.

---

## Repository Structure

```
PweOS/
├── Cargo.toml               # Rust package manifest
├── rust-toolchain.toml      # Nightly toolchain pinning & rust-src
├── .cargo/
│   └── config.toml          # Custom target specification & build-std flags
├── targets/
│   └── i686-pweos.json      # Bare-metal i686 target definition
├── Makefile                 # Build system & QEMU targets
├── README.md                # Documentation & Architecture overview
├── linker.ld                # Linker script (entry at 0x10000)
├── boot/
│   └── boot.asm             # Stage 1 MBR Bootloader (16-bit to 32-bit switch)
└── src/
    ├── lib.rs               # Kernel initialization (kmain) & panic handler
    ├── print.rs             # Formatted print! and println! macros
    ├── sync.rs              # Interrupt-safe spinlock synchronization
    ├── arch/
    │   ├── mod.rs           # Architecture module root
    │   ├── entry.asm        # 32-bit kernel startup & entry stub
    │   ├── cpu.rs           # CPUID hardware feature detection
    │   ├── idt.rs           # Interrupt Descriptor Table management
    │   ├── io.rs            # x86 port I/O (inb, outb, inw, outw, io_wait)
    │   ├── isr.asm          # Low-level interrupt service routine stubs
    │   ├── isr.rs           # Exception and IRQ dispatchers & panic dump
    │   └── pic.rs           # 8259 Programmable Interrupt Controller
    ├── drivers/
    │   ├── mod.rs           # Drivers module root
    │   ├── keyboard.rs      # PS/2 keyboard & serial input with arrow keys
    │   ├── rtc.rs           # CMOS Real-Time Clock driver
    │   ├── serial.rs        # COM1 UART serial port driver (38400 baud, 8N1)
    │   ├── timer.rs         # 8253 PIT timer (100 Hz, uptime, sleep)
    │   └── vga.rs           # VGA 80x25 text mode & hardware cursor
    ├── fs/
    │   ├── mod.rs           # Filesystem module root
    │   └── ramfs.rs         # Dynamic RAM virtual filesystem
    ├── mm/
    │   ├── mod.rs           # Memory management module root
    │   └── heap.rs          # Dynamic kernel heap allocator & GlobalAlloc
    └── shell/
        └── mod.rs           # Interactive shell with history and 22 builtins
```

---

## Prerequisites

To build and run PweOS, you need:
- **Rust** (Nightly channel with `rust-src` component)
- **NASM** (Netwide Assembler)
- **LLD** (`ld.lld` linker)
- **QEMU** (`qemu-system-i386`)

On macOS (via Homebrew):
```bash
brew install nasm lld qemu
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

---

## Building and Running

### Build the OS image
```bash
make
```
This compiles the bootloader and Rust kernel, links the binary, and creates a bootable floppy disk image at `build/pweos.img`.

You can also use Cargo directly:
```bash
cargo check
cargo clippy
cargo build --release
```

### Run in QEMU (Resizable GUI window)
```bash
make run
```
Launches QEMU with `-display cocoa,zoom-to-fit=on` on macOS. The window can be freely resized, dragged by its corner, or maximized using the green window button to scale the VGA screen to any size! You can also press `Cmd+F` or `Ctrl+Alt+F` to toggle full screen.

### Run in Fullscreen
```bash
make run-fullscreen
```
Launches QEMU directly in full screen mode (`-display cocoa,zoom-to-fit=on,full-screen=on`).

### Run in Terminal with Curses (Large Terminal Mode)
```bash
make run-curses
```
Renders the OS display directly inside your current terminal window using curses. The display scales to match your terminal window and font size.

### Run in QEMU with Serial Port Output
```bash
make run-serial
```
Outputs both to the resizable graphical VGA window and mirrors all text to your host terminal via COM1.

### Run Headless (CLI only)
```bash
make run-headless
```
Runs completely in your terminal without opening a graphical window. You can interact directly with the shell using your terminal keyboard!

### Clean build artifacts
```bash
make clean
```
