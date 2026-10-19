# PweOS
PweOS is a minimal, educational 32-bit x86 operating system featuring a custom BIOS bootloader, a freestanding Rust kernel (`no_std`, `alloc`), a dynamic heap memory allocator implementing Rust's `GlobalAlloc`, interrupt-driven device drivers, an in-memory RAMFS, PweFS-backed persistent storage under `/home/`, and an interactive command shell. It also features a pixel-based framebuffer for 320×200, 256-color RGB332 graphics, image rendering, video playback, and real-time drawing demonstrations, alongside TinyCC-powered C compilation for building and running static i386 ELF programs directly within the OS.


```text
       ____              pweos@pweos
      |  _ \             ----------------
      | |_) |            OS: PweOS 0.1.1
      |  __/             Arch: i686
      |_|                Kernel: Rust (no_std)
                         Shell: PweOS
                         Display: VGA 80x25
                         Memory: 2048 KB heap
                         Filesystem: RAMFS + PweFS
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
|          RAMFS + PweFS Virtual Filesystems                   |
| RAMFS for system files; persistent PweFS storage under /home |
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

### 4. Filesystems (`src/fs/ramfs.rs`, `src/fs/pwefs.rs`)
- **RAMFS** is a dynamic in-memory filesystem backed by the heap. It holds default system files, bundled assets, and runtime files outside `/home/`; RAMFS contents are lost on reboot.
- **PweFS** uses the secondary IDE storage disk for persistent user data. The kernel attempts to initialize or mount it automatically during filesystem startup; users do not need a manual `pfs format` or `pfs mount` command.
- **`/home/` is the persistent user-data location.** Files created there are stored on PweFS and are intended to survive reboot. The current PweFS directory-table format supports file and directory entries, including nested paths, subject to its path-length and directory-table limits.
- PweFS only auto-formats a completely blank storage disk. If the disk is nonblank but its filesystem metadata is invalid, initialization should report an error rather than automatically formatting it.
- `/images/` contains bundled RGB332 stills, `/videos/` exposes video clips from the companion data disk, and `/bin/` exposes bundled programs as read-only disk-backed files.

### 5. VGA Text Console & Serial Driver (`src/drivers/vga.rs`, `src/drivers/serial.rs`)
- Memory-mapped text buffer at physical address `0xB8000` (80 columns x 25 rows).
- The `image <path>` command uses 320x200 VGA graphics mode with a 256-color RGB332 palette, showing one source pixel per screen pixel. Paths can be absolute or relative to the current directory. Press Escape to restore text mode and clear the shell screen. Image rendering lives in `src/shell/image.rs` and VGA mode/palette handling lives in `src/drivers/vga.rs`.
- The `video <path>` command plays 320x200 RGB332 clips from `/videos/`. PWEV files use periodic run-length keyframes plus changed-pixel delta frames. Video data lives on the companion IDE data disk, so it does not consume the kernel boot image's 448 KiB load limit. Press any key to return to the shell. Playback uses the shared framebuffer in `src/drivers/framebuffer.rs`.
- The companion IDE disk stores the static ELF programs under `/bin` and the TinyCC compiler. RAMFS exposes them as read-only files and loads their bytes on demand, keeping executable images out of the boot kernel.
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
- **ATA PIO (`ata.rs`)**: Provides sector access to IDE disks. The primary-master companion data disk supplies video assets and user programs; the primary-slave storage disk backs persistent PweFS data under `/home/`.
- **CPUID Detection (`cpu.rs`)**: Queries CPU vendor string, processor model/brand string, family/stepping, and hardware capability flags.

### 8. Interactive Shell (`src/shell/`)
- Colored prompt (`pweos> `) with backspace character deletion.
- **Command History**: Remembers up to 16 previous commands. Press **Up Arrow** and **Down Arrow** to cycle through history!
- **Tab Completion**: Complete command names and RAMFS file or directory paths. A unique match completes automatically; ambiguous matches extend to the shared prefix or show matching names.
- Interactive command interpreter with line editing, builtin commands, and static i386 ELF programs.
- Initial user-mode runtime: identity paging separates a 4 MiB-and-above user arena from kernel memory, a ring-3 GDT/TSS path enters user code, and `int 0x80` provides console, process, RAMFS, and heap calls.
- Static i386 ELF execution: bundled programs can be run by name from `/bin` (for example, `hello`, `calc 2 + 3`, or `vi notes.txt`). The shell also runs a file from the current directory, adding `.elf` when needed. `exec <path> [arguments...]` runs an ELF at an explicit path. Programs can use the syscall wrappers in `examples/pweos.h`; ELF files must be static and linked into the user arena. Dynamic ELF files are not supported.
- Bundled user programs include `hello`, `echo`, `calc`, `cat`, `grep`, and `vi`. The editor supports basic vi-style movement, insert mode, line deletion, and `:w`, `:q`, `:q!`, and `:wq` commands.
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
pweos> hello
Hello from PweOS!
```

`tcc -v` prints compiler information, and `tcc -c hello.c -o hello.o` creates an object file. Object files are not executable with `exec`; omit `-c` to link a runnable program. The shell supplies PweOS's startup code and runtime when linking. Programs must be static i386 ELF files and use the bundled PweOS C headers and runtime; host libraries and dynamic linking are not available.

---

## Persistent User Storage

PweOS reserves `/home/` for persistent user files and directories. PweFS is initialized automatically during startup when the storage disk is available.

```text
pweos> mkdir /home/prerak
pweos> write /home/prerak/hello.txt Hello from PweOS!
pweos> mkdir /home/prerak/projects
pweos> write /home/prerak/projects/todo.txt Build PweOS
pweos> ls /home/prerak
pweos> cat /home/prerak/hello.txt
```

Files and directories saved under `/home/` are intended to survive reboot. Other ordinary RAMFS files are temporary. Bundled `/bin/` programs and `/videos/` assets remain on the companion data disk and are exposed read-only by the filesystem layer.

There is no user-facing `pfs` shell command: normal startup handles mounting or initializing storage. A completely blank storage disk may be initialized automatically. Nonblank storage with invalid metadata should not be automatically reformatted. Keep the storage image (`build/storage.img`) intact when rebuilding if you want to preserve user data.

PweFS uses a fixed on-disk directory table, so the number of entries and path length are limited. If storage initialization fails, the kernel reports that persistent storage is unavailable; RAMFS can still provide the system's in-memory files.

---

## Builtin Shell Commands

| Command | Syntax | Description |
|---|---|---|
| `help` | `help` | Lists all available commands and their descriptions |
| `clear` | `clear` | Clears the VGA text screen and resets cursor position |
| `echo` | `echo [text...]` | Prints the arguments to standard output; bundled user program |
| `version` / `uname` | `version` | Displays OS name, version, and target architecture |
| `uptime` | `uptime` | Displays uptime in minutes and seconds, along with timer ticks |
| `date` | `date` | Reads and displays current CMOS Real-Time Clock date and time |
| `cpuinfo` | `cpuinfo` | Queries CPUID to display vendor, brand string, and feature flags |
| `meminfo` | `meminfo` | Displays total heap capacity, used bytes, free bytes, and blocks |
| `history` | `history` | Displays previous commands stored in the shell history buffer |
| `ls` | `ls [path]` | Lists the current directory or a selected path; directory names end in `/` |
| `mkdir` | `mkdir <path>` | Creates a directory under an existing parent directory; directories under `/home/` are stored persistently |
| `rmdir` | `rmdir <path>` | Removes an empty directory |
| `nuke` | `nuke <path>` | Recursively removes a file or directory and its contents; protects the root, current directory, and mounted `/bin`, `/videos`, and `/home` mount points |
| `cd` | `cd <path>` | Changes the working directory; supports `..` and absolute paths |
| `pwd` | `pwd` | Prints the current working directory |
| `cat` | `cat [file...]` | Writes file contents or piped input to standard output; bundled user program |
| `exec` | `exec <path> [arguments...]` | Runs a static i386 ELF executable from RAMFS, including disk-backed `/bin` files |
| `hello` | `hello [arguments...]` | Example user program, bundled as a static i386 ELF |
| `tcc` | `tcc [options] <file.c>` | Compiles C source in RAMFS to a static i386 ELF; e.g. `tcc hello.c -o hello`, then run it with `hello` |
| `image` | `image <path>` | Displays a 320x200 RGB332 file at 256 colors; e.g. `image /images/cat.rgb332` or `image billi.rgb332` from `/images/` |
| `video` | `video <path>` | Plays a bundled `.pwevid` clip, e.g. `/videos/demo.pwevid`; press any key to stop and return to the shell |
| `cube` | `cube` | Shows a spinning colored wireframe cube; press any key to return to the shell |
| `grep` | `grep <text> [file...]` | Prints matching lines from files or piped input; bundled user program |
| `touch` | `touch <filename>` | Creates an empty file; files under `/home/` are persistent, while other writable paths use RAMFS |
| `write` | `write <filename> <text>` | Replaces a file with one line of text; files under `/home/` are persistent |
| `edit` | `edit [path]` | Opens a multiline editor; Tab inserts four spaces, Escape saves, and piped text prompts for a save path. Save under `/home/` for persistence |
| `vi` | `vi <path>` | Opens the bundled user-mode vi-style editor: `h/j/k/l` move, `i` inserts, `o/O` open lines, `x` and `dd` delete, `Esc` returns to normal mode, and `:w`, `:q`, `:q!`, `:wq` save or quit |
| `less` | `less [path]` | Views a file or piped text without editing; arrows navigate and `q` or Escape exits |
| `rm` | `rm <filename>` | Deletes a file; files under `/home/` are deleted from persistent storage |
| `calc` | `calc <a> <op> <b>` | Performs arithmetic: `+`, `-`, `*`, `/`, `%` with div-by-zero protection; bundled user program |
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
```text
PweOS/
├── Cargo.toml               # Rust package manifest
├── rust-toolchain.toml      # Nightly toolchain pinning & rust-src
├── .cargo/
│   └── config.toml          # Custom target specification & build-std flags
├── targets/
│   └── i686-pweos.json      # Bare-metal i686 target definition
├── Makefile                 # Build system, ISO generation & QEMU targets
├── README.md                # Documentation & architecture overview
├── linker.ld                # Linker script (entry at 0x10000)
├── build.rs                 # Bundles image and video metadata
├── assets/                  # RGB332 stills and PWEV clips
├── tools/
│   ├── image_to_rgb332.py   # Convert still images for the OS
│   ├── video_to_pwe.py      # Convert video clips for the OS
│   └── pack_data_disk.py    # Build the companion data disk
├── boot/
│   └── boot.asm             # Stage 1 bootloader (16-bit to 32-bit switch)
└── src/
    ├── lib.rs               # Kernel initialization (kmain) & panic handler
    ├── print.rs             # Formatted print! and println! macros
    ├── sync.rs              # Interrupt-safe spinlock synchronization
    ├── arch/
    │   ├── mod.rs           # Architecture module root
    │   ├── entry.asm        # 32-bit kernel startup & entry stub
    │   ├── cpu.rs           # CPUID hardware feature detection
    │   ├── idt.rs           # Interrupt Descriptor Table management
    │   ├── io.rs            # x86 port I/O operations
    │   ├── isr.asm          # Low-level interrupt service routine stubs
    │   ├── isr.rs           # Exception and IRQ dispatchers & panic dump
    │   └── pic.rs           # 8259 Programmable Interrupt Controller
    ├── drivers/
    │   ├── mod.rs           # Drivers module root
    │   ├── ata.rs           # ATA PIO disk access
    │   ├── atapi.rs         # ATAPI CD-ROM sector reader
    │   ├── disk.rs          # Unified disk reader with ISO data disk fallback
    │   ├── framebuffer.rs   # VGA RGB332 pixel, line, and triangle drawing
    │   ├── keyboard.rs      # PS/2 keyboard & serial input with arrow keys
    │   ├── rtc.rs           # CMOS Real-Time Clock driver
    │   ├── serial.rs        # COM1 UART serial port driver (38400 baud, 8N1)
    │   ├── timer.rs         # 8253 PIT timer (100 Hz, uptime, sleep)
    │   └── vga.rs           # VGA 80x25 text mode & hardware cursor
    ├── fs/
    │   ├── mod.rs           # Filesystem module root
    │   ├── ramfs.rs         # Dynamic RAM virtual filesystem and /home routing
    │   └── pwefs.rs         # Persistent filesystem backing /home
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

The converter writes `assets/clip.pwevid` by default. Add every `.pwevid` directly inside `assets/`; the build packs them into `build/data.img` and lists them under `/videos/`. The player reads one encoded frame at a time from this companion disk, so full clips do not need to fit in kernel memory. PWEV stores up to 65,535 frames; at 10 FPS that is about 109 minutes. The format is 320x200 RGB332 with periodic keyframes and frame-to-frame deltas. To make a shorter clip, add `--seconds 30`. Install ffmpeg on the host first. After rebuilding, play it with `video /videos/clip.pwevid`; press any key during playback to return to the shell.

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

### Build a Bootable ISO
```bash
make iso
```
Builds PweOS and packages the bootloader, kernel, and data disk into a bootable ISO image.

### Run ISO in QEMU
```bash
make run-iso
```
Builds the ISO if needed and launches QEMU to boot PweOS directly from the ISO in a graphical window.

### Run ISO in Fullscreen
```bash
make run-iso-fullscreen
```
Builds the ISO if needed and launches QEMU directly in full screen mode (`-display cocoa,zoom-to-fit=on,full-screen=on`).

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
