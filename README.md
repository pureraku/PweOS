# PweOS

PweOS is a minimal, educational 32-bit x86 operating system featuring a custom BIOS bootloader, freestanding Rust kernel (`no_std`, `alloc`), dynamic heap memory allocator implementing Rust's `GlobalAlloc`, interrupt-driven device drivers, in-memory virtual filesystem, and an interactive command shell.

```text
       ____              pweos@pweos
      |  _ \             ----------------
      | |_) |            OS: PweOS 0.1.0
      |  __/             Arch: i686
      |_|                Kernel: Rust (no_std)
                         Shell: PweOS
                         Display: VGA 80x25
                         Memory: 2048 KB heap
                         Filesystem: RAMFS
                         Compiler: TinyCC (i386)
                         CPU mode: 32-bit protected mode

Type 'help' to see available commands.
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
|  VGA 80x25 Text | PS/2 Keyboard | PIT 8253 | CMOS RTC | ATA PIO | COM1|
+-------------------------------------------------------------+
|                 x86 Interrupt Subsystem                     |
|  256-Entry IDT | 8259 PIC Remapper | ISR Stubs & Panic Dump  |
+-------------------------------------------------------------+
|                    32-bit Protected Mode                    |
|   GDT (Flat 4GB Code & Data) | A20 Gate | Stack at 0x90000 |
+-------------------------------------------------------------+
|              Stage 1 BIOS Boot Sector (512 Bytes)           |
|      Loads kernel sectors from floppy disk via BIOS INT 13h |
+-------------------------------------------------------------+
```

### 1. Bootloader (`boot/boot.asm`)
- **Stage 1 MBR Boot Sector (512 bytes)** executed by BIOS at `0x7C00`.
- Performs **LBA to CHS conversion** to load up to 448 KB of kernel data into physical memory starting at `0x10000` (`0x1000:0x0000`), advancing `ES` segment by `0x20` per sector. This keeps the kernel below the VGA memory window.
- Enables the **A20 Address Line** via Fast A20 gate.
- Loads a 32-bit **Global Descriptor Table (GDT)** defining flat 4GB code and data segments.
- Transitions the processor from 16-bit Real Mode to **32-bit Protected Mode** (`CR0.PE = 1`) and jumps to kernel entry.

### 2. Kernel Entry & Linker (`src/arch/entry.asm`, `linker.ld`)
- Sets up 32-bit data segment registers (`DS`, `ES`, `FS`, `GS`, `SS`).
- Initializes a dedicated stack at `0x90000`, above the kernel load area.
- Transfers control to Rust kernel entry point `kmain()`.

### 3. Dynamic Heap Memory Manager (`src/mm/heap.rs`)
- Manages a 2 MB dynamic heap starting at `0x100000` (1 MB mark).
- Linked-list block allocator with 8-byte alignment, block splitting, and automatic boundary coalescing on free.
- Implements Rust's `core::alloc::GlobalAlloc` trait, enabling full use of the standard `alloc` crate (`String`, `Vec`, `Box`, `format!`).
- Provides live memory diagnostics via `meminfo`.

### 4. RAMFS Virtual Filesystem (`src/fs/ramfs.rs`)
- Dynamic in-memory virtual filesystem backed by the heap.
- Supports file creation, resizing, reading, appending, and deletion.
- Prepopulated with default text files, an `/images/` directory for RGB332 stills, and a `/videos/` directory for PweOS video clips stored on a companion data disk.

### 5. VGA Text Console & Serial Driver (`src/drivers/vga.rs`, `src/drivers/serial.rs`)
- Memory-mapped text buffer at physical address `0xB8000` (80 columns x 25 rows).
- The `image <path>` command uses 320x200 VGA graphics mode with a 256-color RGB332 palette, showing one source pixel per screen pixel. Paths can be absolute or relative to the current directory. Press Escape to restore text mode and clear the shell screen. Image rendering lives in `src/shell/image.rs` and VGA mode/palette handling lives in `src/drivers/vga.rs`.
- The `video <path>` command plays 320x200 RGB332 clips from `/videos/`. PWEV files use periodic run-length keyframes plus changed-pixel delta frames. Video data lives on a separate raw IDE disk image, so it does not consume the kernel boot image's 448 KiB load limit. Press any key to return to the shell. Playback uses the shared framebuffer in `src/drivers/framebuffer.rs`.
- The companion IDE disk also stores `/bin/tcc.elf`; PweOS reads it on demand so the compiler binary does not cross the VGA memory window during boot.
- Includes `/videos/demo.pwevid`, a short moving color-block clip for trying the player immediately.
- The `cube` command demonstrates the `Framebuffer` pixel and line API in `src/drivers/framebuffer.rs`. It continuously rotates a colored wireframe cube in the 320x200 RGB332 framebuffer; press any key to restore text mode and return to the shell.
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
- **ATA PIO (`ata.rs`)**: Reads video assets from the companion primary-master IDE disk without loading large video files into the kernel image.
- **CPUID Detection (`cpu.rs`)**: Queries CPU vendor string, processor model/brand string, family/stepping, and hardware capability flags.

### 8. Interactive Shell (`src/shell/`)
- Colored prompt (`pweos> `) with backspace character deletion.
- **Command History**: Remembers up to 16 previous commands. Press **Up Arrow** and **Down Arrow** to cycle through history!
- **Tab Completion**: Complete command names and RAMFS file or directory paths. A unique match completes automatically; ambiguous matches extend to the shared prefix or show matching names.
- Interactive command interpreter with line editing and builtin commands.
- Initial user-mode runtime: identity paging separates a 4 MiB-and-above user arena from kernel memory, a ring-3 GDT/TSS path enters user code, and `int 0x80` provides console, process, RAMFS, and heap calls.
- Static i386 ELF execution: `exec /bin/hello.elf` runs a small freestanding C program packaged in RAMFS. Programs can use the syscall wrappers in `examples/pweos.h`; ELF files must be static and linked into the user arena. Dynamic ELF files are not supported.
- `vi <path>` launches the statically linked i386 editor from RAMFS as a user program. It supports basic vi-style movement, insert mode, line deletion, and `:w`, `:q`, `:q!`, and `:wq` commands.
- TinyCC is available as the `tcc` shell command. It compiles C source files stored in RAMFS into static i386 ELF programs for `exec`; it is built into the companion disk image and loaded on demand.

### Compiling C programs with TinyCC

Create a C source file with the shell editor, compile it, and run the resulting ELF:

```text
pweos> edit hello.c
```

Enter and save:

```c
#include <stdio.h>

int main(void) {
    printf("Hello from PweOS!\n");
    return 0;
}
```

Then compile and execute it:

```text
pweos> tcc hello.c -o hello
TinyCC exited with status 0.
pweos> exec hello
Hello from PweOS!
Program exited with status 0.
```

`tcc -v` prints compiler information, and `tcc -c hello.c -o hello.o` creates an object file. Object files are not executable with `exec`; omit `-c` to link a runnable program. The shell supplies PweOS's startup code and runtime when linking. Programs must be static i386 ELF files and use the bundled PweOS C headers and runtime; host libraries and dynamic linking are not available.

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
| `exec` | `exec <path>` | Runs a static i386 ELF executable from RAMFS; try `exec /bin/hello.elf` |
| `tcc` | `tcc [options] <file.c>` | Compiles C source in RAMFS to a static i386 ELF; e.g. `tcc hello.c -o hello`, then `exec hello` |
| `image` | `image <path>` | Displays a 320x200 RGB332 file at 256 colors; e.g. `image /images/cat.rgb332` or `image billi.rgb332` from `/images/` |
| `video` | `video <path>` | Plays a bundled `.pwevid` clip, e.g. `/videos/demo.pwevid`; press any key to stop and return to the shell |
| `cube` | `cube` | Shows a spinning colored wireframe cube; press any key to return to the shell |
| `grep` | `grep <text> [path...]` | Prints matching lines from files or piped input |
| `touch` | `touch <filename>` | Creates an empty file in RAMFS |
| `write` | `write <filename> <text>` | Replaces a file with one line of text |
| `edit` | `edit [path]` | Opens a multiline editor; Tab inserts four spaces, Escape saves, and piped text prompts for a save path |
| `vi` | `vi <path>` | Opens a vi-style editor: `h/j/k/l` move, `i` inserts, `o/O` open lines, `x` and `dd` delete, `Esc` returns to normal mode, and `:w`, `:q`, `:q!`, `:wq` save or quit |
| `less` | `less [path]` | Views a file or piped text without editing; arrows navigate and `q` or Escape exits |
| `rm` | `rm <filename>` | Deletes a file from RAMFS and reclaims memory |
| `calc` | `calc <a> <op> <b>` | Performs arithmetic: `+`, `-`, `*`, `/`, `%` with div-by-zero protection |
| `color` | `color <fg> [bg]` | Changes console foreground and optional background color |
| `sleep` | `sleep <seconds>` | Delays execution for N seconds using timer interrupts |
| `peek` | `peek <hex_addr>` | Reads and displays the byte stored at physical memory address |
| `reboot` | `reboot` | Restarts the machine via 8042 keyboard controller reset line |
| `poweroff` | `poweroff` | Powers off QEMU/Bochs through their ACPI shutdown ports; otherwise halts the CPU |
| `panic` | `panic` | Triggers a division-by-zero exception to demonstrate the crash handler |
| `about` | `about` | Displays system architecture and component specifications |

Commands can be connected with `|` to pass output to the next command. For example, `cat notes.txt | grep TODO` filters a file, `cat notes.txt | write copy.txt` copies it, `help | less` views command help, and `help | edit` opens that help text for editing and asks where to save it when you press Escape.

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
├── build.rs                 # Bundles image and video metadata
├── assets/                  # RGB332 stills and PWEV clips
├── tools/
│   ├── image_to_rgb332.py   # Convert still images for the OS
│   ├── video_to_pwe.py      # Convert video clips for the OS
│   └── pack_videos.py       # Build the companion video disk
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
    │   ├── ata.rs           # Primary-master ATA PIO reads for video storage
    │   ├── framebuffer.rs   # VGA RGB332 pixel, line, and triangle drawing
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
        ├── video.rs         # PWEV frame decoding and playback
        └── mod.rs           # Interactive shell and builtin commands
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

### Add an image

Convert a JPEG or another Pillow-supported image to the PweOS RGB332 format:

```bash
uv run tools/image_to_rgb332.py path/to/photo.jpg
```

The converter writes `assets/photo.rgb332` at 320x200, preserving the source aspect ratio with black padding where needed. Use `--output assets/custom-name.rgb332` to choose another bundled name. Every `.rgb332` file directly inside `assets/` is bundled into RAMFS automatically on the next build and appears under `/images/`; view it with `image /images/photo.rgb332`. The first `uv run` may download Pillow.

### Add a video

Convert a video supported by ffmpeg to PweOS's compact framebuffer format. By default, the converter processes the full video:

```bash
uv run tools/video_to_pwe.py path/to/clip.mp4 --fps 10
```

The converter writes `assets/clip.pwevid` by default. Add every `.pwevid` directly inside `assets/`; the build packs them into `build/videos.img` and lists them under `/videos/`. The player reads one encoded frame at a time from this companion disk, so full clips do not need to fit in kernel memory. PWEV stores up to 65,535 frames; at 10 FPS that is about 109 minutes. The format is 320x200 RGB332 with periodic keyframes and frame-to-frame deltas. To make a shorter clip, add `--seconds 30`. Install ffmpeg on the host first. After rebuilding, play it with `video /videos/clip.pwevid`; press any key during playback to return to the shell.

Try the included animation first with `video /videos/demo.pwevid`.

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
> This project was heavily assisted by AI for educational purposes if you have any suggestions please send PRs
