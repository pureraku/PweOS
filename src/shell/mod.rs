extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use crate::arch::cpu::cpu_get_info;
use crate::arch::io::{inb, outb};
use crate::drivers::keyboard::{keyboard_getchar, KEY_DOWN, KEY_UP};
use crate::drivers::rtc::rtc_get_datetime;
use crate::drivers::timer::{timer_get_ticks, timer_get_uptime_seconds, timer_sleep};
use crate::drivers::vga::{vga_clear, vga_putchar, vga_set_color, VgaColor};
use crate::fs::ramfs::{
    ramfs_count, ramfs_create, ramfs_delete, ramfs_list, ramfs_read, ramfs_write,
};
use crate::mm::heap::heap_get_stats;

const PWEOS_VERSION: &str = "0.1.0";
const SHELL_HISTORY_MAX: usize = 16;

struct ShellHistory {
    commands: Vec<String>,
}

impl ShellHistory {
    const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    fn add(&mut self, cmd: &str) {
        if cmd.is_empty() {
            return;
        }
        if let Some(last) = self.commands.last() {
            if last == cmd {
                return;
            }
        }
        if self.commands.len() >= SHELL_HISTORY_MAX {
            self.commands.remove(0);
        }
        self.commands.push(String::from(cmd));
    }

    fn count(&self) -> usize {
        self.commands.len()
    }

    fn get(&self, idx: usize) -> Option<&str> {
        self.commands.get(idx).map(|s| s.as_str())
    }
}

use crate::sync::Spinlock;

static HISTORY: Spinlock<ShellHistory> = Spinlock::new(ShellHistory::new());

struct Command {
    name: &'static str,
    desc: &'static str,
    handler: fn(args: &[&str]),
}

static COMMANDS: &[Command] = &[
    Command {
        name: "help",
        desc: "Display list of available commands",
        handler: cmd_help,
    },
    Command {
        name: "clear",
        desc: "Clear screen and reset cursor",
        handler: cmd_clear,
    },
    Command {
        name: "echo",
        desc: "Print arguments to the console",
        handler: cmd_echo,
    },
    Command {
        name: "version",
        desc: "Display PweOS kernel version and arch",
        handler: cmd_version,
    },
    Command {
        name: "uname",
        desc: "Display operating system information",
        handler: cmd_version,
    },
    Command {
        name: "uptime",
        desc: "Display system uptime and timer ticks",
        handler: cmd_uptime,
    },
    Command {
        name: "date",
        desc: "Display real-time clock (CMOS RTC) date",
        handler: cmd_date,
    },
    Command {
        name: "cpuinfo",
        desc: "Display CPU hardware details via CPUID",
        handler: cmd_cpuinfo,
    },
    Command {
        name: "meminfo",
        desc: "Display kernel heap memory statistics",
        handler: cmd_meminfo,
    },
    Command {
        name: "history",
        desc: "Display command history buffer",
        handler: cmd_history,
    },
    Command {
        name: "ls",
        desc: "List files in in-memory RAM filesystem",
        handler: cmd_ls,
    },
    Command {
        name: "cat",
        desc: "Display file contents: cat <filename>",
        handler: cmd_cat,
    },
    Command {
        name: "touch",
        desc: "Create empty file: touch <filename>",
        handler: cmd_touch,
    },
    Command {
        name: "write",
        desc: "Write to file: write <filename> <text>",
        handler: cmd_write,
    },
    Command {
        name: "rm",
        desc: "Delete file: rm <filename>",
        handler: cmd_rm,
    },
    Command {
        name: "calc",
        desc: "Evaluate arithmetic expression: a op b",
        handler: cmd_calc,
    },
    Command {
        name: "color",
        desc: "Set terminal color: color <fg> [bg]",
        handler: cmd_color,
    },
    Command {
        name: "sleep",
        desc: "Delay execution for N seconds",
        handler: cmd_sleep,
    },
    Command {
        name: "peek",
        desc: "Read byte from memory: peek <hex_addr>",
        handler: cmd_peek,
    },
    Command {
        name: "reboot",
        desc: "Reboot the system",
        handler: cmd_reboot,
    },
    Command {
        name: "panic",
        desc: "Trigger kernel panic (exception test)",
        handler: cmd_panic,
    },
    Command {
        name: "about",
        desc: "Information about PweOS system",
        handler: cmd_about,
    },
    Command {
        name: "whoami",
        desc: "Prints user",
        handler: cmd_whoami,
    },
];

fn cmd_whoami(_args: &[&str]){
    println!("idk you tell me");
}

fn cmd_help(_args: &[&str]) {
    println!("\nPweOS Builtin Commands:");
    println!("-----------------------:3---------------------------");
    for cmd in COMMANDS {
        vga_set_color(VgaColor::LightGreen, VgaColor::Black);
        print!("  {:<10}", cmd.name);
        vga_set_color(VgaColor::LightGrey, VgaColor::Black);
        println!(" - {}", cmd.desc);
    }
    println!("-----------------------:3----------------------------");
}

fn cmd_clear(_args: &[&str]) {
    vga_clear();
}

fn cmd_echo(args: &[&str]) {
    for (i, &arg) in args.iter().enumerate().skip(1) {
        print!("{}{}", arg, if i == args.len() - 1 { "" } else { " " });
    }
    println!();
}

fn cmd_version(_args: &[&str]) {
    println!("PweOS :3 version {} (i686-pc-none-elf)", PWEOS_VERSION);
    println!("Build: Freestanding 32-bit x86 Kernel (Rust)");
}

fn cmd_uptime(_args: &[&str]) {
    let ticks = timer_get_ticks();
    let secs = timer_get_uptime_seconds();
    let mins = secs / 60;
    let rem_secs = secs % 60;

    println!(
        "Uptime: {} min, {} sec ({} timer ticks elapsed at 100 Hz) :3",
        mins, rem_secs, ticks
    );
}

fn cmd_date(_args: &[&str]) {
    let dt = rtc_get_datetime();
    println!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC :3",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second
    );
}

fn cmd_cpuinfo(_args: &[&str]) {
    let info = cpu_get_info();

    println!("\nCPU Hardware Information:");
    println!("----------------------------------------------------");
    println!("  Vendor ID        : {}", info.vendor_str());
    println!("  Model / Brand    : {}", info.brand_str());
    println!(
        "  Family / Model   : Family {}, Model {}, Stepping {}",
        info.family, info.model, info.stepping
    );
    print!("  Features         : ");
    if info.has_fpu { print!("FPU "); }
    if info.has_tsc { print!("TSC "); }
    if info.has_msr { print!("MSR "); }
    if info.has_apic { print!("APIC "); }
    if info.has_sse { print!("SSE "); }
    if info.has_sse2 { print!("SSE2 "); }
    println!();
    println!(
        "  Hypervisor State : {}",
        if info.has_hypervisor {
            "Virtual Machine Detected"
        } else {
            "Bare Metal"
        }
    );
    println!("----------------------------------------------------");
}

fn cmd_ls(_args: &[&str]) {
    println!("\nRAMFS Files ({} total):", ramfs_count());
    println!("dang so many files..");
    ramfs_list(|name, size| {
        vga_set_color(VgaColor::LightCyan, VgaColor::Black);
        print!("  {:<16}", name);
        vga_set_color(VgaColor::LightGrey, VgaColor::Black);
        println!(" {} bytes", size);
    });
    println!("----------------------------------------------------");
}

fn cmd_cat(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: cat <filename>");
        return;
    }
    match ramfs_read(args[1]) {
        Some(data) => {
            if let Ok(text) = core::str::from_utf8(&data) {
                print!("{}", text);
                if !data.ends_with(b"\n") {
                    println!();
                }
            } else {
                for &b in &data {
                    vga_putchar(b);
                }
                if !data.ends_with(b"\n") {
                    println!();
                }
            }
        }
        None => {
            vga_set_color(VgaColor::LightRed, VgaColor::Black);
            println!("Error: File '{}' not found.", args[1]);
            vga_set_color(VgaColor::LightGrey, VgaColor::Black);
        }
    }
}

fn cmd_touch(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: touch <filename>");
        return;
    }
    if ramfs_create(args[1], b"") {
        println!("Created file '{}'. :3", args[1]);
    } else {
        println!("File '{}' already exists or creation failed.", args[1]);
    }
}

fn cmd_write(args: &[&str]) {
    if args.len() < 3 {
        println!("Usage: write <filename> <text...>");
        return;
    }

    let mut content = String::new();
    for (i, &arg) in args.iter().enumerate().skip(2) {
        if i > 2 {
            content.push(' ');
        }
        content.push_str(arg);
    }
    content.push('\n');

    let len = content.len();
    if ramfs_write(args[1], content.as_bytes(), false) {
        println!("Wrote {} bytes to '{}'. :3", len, args[1]);
    } else {
        println!("Failed to write to '{}'. :(", args[1]);
    }
}

fn cmd_rm(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: rm <filename>");
        return;
    }
    if ramfs_delete(args[1]) {
        println!("Deleted file '{}'. :3", args[1]);
    } else {
        vga_set_color(VgaColor::LightRed, VgaColor::Black);
        println!("Error: File '{}' not found.", args[1]);
        vga_set_color(VgaColor::LightGrey, VgaColor::Black);
    }
}

fn cmd_meminfo(_args: &[&str]) {
    let stats = heap_get_stats();

    println!("\nKernel Heap Memory Statistics:");
    println!("----------------------------------------------------");
    println!(
        "  Total Heap Capacity : {} KB ({} bytes)",
        stats.total_bytes / 1024,
        stats.total_bytes
    );
    println!("  Used Memory         : {} bytes", stats.used_bytes);
    println!("  Free Memory         : {} bytes", stats.free_bytes);
    println!("  Active Allocations  : {}", stats.num_allocations);
    println!("  Total Heap Blocks   : {}", stats.num_blocks);
    println!("----------------------------------------------------");
}

fn parse_i32(s: &str) -> Option<i32> {
    let mut num: i32 = 0;
    let mut negative = false;
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut i = 0;
    if bytes[0] == b'-' {
        negative = true;
        i += 1;
    } else if bytes[0] == b'+' {
        i += 1;
    }

    if i >= bytes.len() {
        return None;
    }

    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            return None;
        }
        num = num.wrapping_mul(10).wrapping_add((b - b'0') as i32);
        i += 1;
    }

    Some(if negative { -num } else { num })
}

fn cmd_calc(args: &[&str]) {
    if args.len() < 4 {
        println!("Usage: calc <number> <+|-|*|/|%> <number>");
        println!("Example: calc 15 * 7");
        return;
    }

    let a = match parse_i32(args[1]) {
        Some(v) => v,
        None => {
            println!("Invalid number '{}'", args[1]);
            return;
        }
    };
    let op = args[2];
    let b = match parse_i32(args[3]) {
        Some(v) => v,
        None => {
            println!("Invalid number '{}'", args[3]);
            return;
        }
    };

    match op {
        "+" => {
            let res = a.wrapping_add(b);
            println!("{} + {} = {}", a, b, res);
        }
        "-" => {
            let res = a.wrapping_sub(b);
            println!("{} - {} = {}", a, b, res);
        }
        "*" => {
            let res = a.wrapping_mul(b);
            println!("{} * {} = {}", a, b, res);
        }
        "/" => {
            if b == 0 {
                vga_set_color(VgaColor::LightRed, VgaColor::Black);
                println!("Error: Division by zero!");
                vga_set_color(VgaColor::LightGrey, VgaColor::Black);
                return;
            }
            let res = a / b;
            println!("{} / {} = {} (remainder {})", a, b, res, a % b);
        }
        "%" => {
            if b == 0 {
                vga_set_color(VgaColor::LightRed, VgaColor::Black);
                println!("Error: Division by zero!");
                vga_set_color(VgaColor::LightGrey, VgaColor::Black);
                return;
            }
            let res = a % b;
            println!("{} % {} = {}", a, b, res);
        }
        _ => {
            println!("Unknown operator '{}'. Supported: +, -, *, /, %", op);
        }
    }
}

fn parse_color_name(name: &str) -> Option<VgaColor> {
    match name {
        "black" => Some(VgaColor::Black),
        "blue" => Some(VgaColor::Blue),
        "green" => Some(VgaColor::Green),
        "cyan" => Some(VgaColor::Cyan),
        "red" => Some(VgaColor::Red),
        "magenta" => Some(VgaColor::Magenta),
        "brown" => Some(VgaColor::Brown),
        "grey" => Some(VgaColor::LightGrey),
        "darkgrey" => Some(VgaColor::DarkGrey),
        "lightblue" => Some(VgaColor::LightBlue),
        "lightgreen" => Some(VgaColor::LightGreen),
        "lightcyan" => Some(VgaColor::LightCyan),
        "lightred" => Some(VgaColor::LightRed),
        "lightmagenta" => Some(VgaColor::LightMagenta),
        "yellow" => Some(VgaColor::Yellow),
        "white" => Some(VgaColor::White),
        _ => None,
    }
}

fn cmd_color(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: color <fg> [bg]");
        println!("Colors: black, blue, green, cyan, red, magenta, brown, grey,");
        println!("        darkgrey, lightblue, lightgreen, lightcyan, lightred,");
        println!("        lightmagenta, yellow, white");
        return;
    }

    let fg = match parse_color_name(args[1]) {
        Some(c) => c,
        None => {
            println!("Unknown foreground color '{}'", args[1]);
            return;
        }
    };

    let bg = if args.len() >= 3 {
        match parse_color_name(args[2]) {
            Some(c) => c,
            None => {
                println!("Unknown background color '{}'", args[2]);
                return;
            }
        }
    } else {
        VgaColor::Black
    };

    vga_set_color(fg, bg);
    println!(
        "Color changed to {} on {}. :3",
        args[1],
        if args.len() >= 3 { args[2] } else { "black" }
    );
}

fn cmd_sleep(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: sleep <seconds>");
        return;
    }
    let s = parse_i32(args[1]).unwrap_or(1);
    let s = if s <= 0 { 1 } else { s };
    println!("Sleeping for {} second(s)... :3", s);
    timer_sleep((s as u32) * 100);
    println!("Done. :3");
}

fn parse_hex(s: &str) -> u32 {
    let mut s = s;
    if s.starts_with("0x") || s.starts_with("0X") {
        s = &s[2..];
    }
    let mut val: u32 = 0;
    for &b in s.as_bytes() {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u32,
            b'a'..=b'f' => (b - b'a' + 10) as u32,
            b'A'..=b'F' => (b - b'A' + 10) as u32,
            _ => break,
        };
        val = (val << 4) | digit;
    }
    val
}

fn cmd_peek(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: peek <hex_address>");
        println!("Example: peek 0xb8000");
        return;
    }
    let addr = parse_hex(args[1]);
    let ptr = addr as *const u8;
    let val = unsafe { ptr.read_volatile() };
    let c = if (32..=126).contains(&val) { val as char } else { '.' };
    println!("Memory [0x{:08x}] = 0x{:02x} ('{}')", addr, val, c);
}

fn cmd_reboot(_args: &[&str]) {
    println!("Restarting PweOS :3...");
    timer_sleep(20);

    // Pulse CPU reset line via 8042 keyboard controller
    unsafe {
        loop {
            let temp = inb(0x64);
            if (temp & 1) != 0 {
                inb(0x60);
            }
            if (temp & 2) == 0 {
                break;
            }
        }
        outb(0x64, 0xFE);

        // Fallback: triple fault via null IDT
        #[repr(C, packed)]
        struct NullIdt {
            limit: u16,
            base: u32,
        }
        let null_idt = NullIdt { limit: 0, base: 0 };
        core::arch::asm!(
            "cli",
            "lidt [{}]",
            "int 3",
            in(reg) &null_idt,
            options(noreturn)
        );
    }
}

fn cmd_panic(_args: &[&str]) {
    println!("Triggering division by zero exception...");
    unsafe {
        core::arch::asm!(
            "mov eax, 42",
            "xor edx, edx",
            "xor ecx, ecx",
            "idiv ecx"
        );
    }
}

fn cmd_about(_args: &[&str]) {
    vga_set_color(VgaColor::LightCyan, VgaColor::Black);
    println!("\n  ==== About PweOS :3 ====");
    vga_set_color(VgaColor::White, VgaColor::Black);
    println!("  Architecture : 32-bit x86 (i686 Protected Mode)");
    println!("  Bootloader   : 2-stage custom BIOS MBR with A20 & GDT");
    println!("  Kernel       : Freestanding Rust runtime (ported from C)");
    println!("  Drivers      : VGA Text Mode, PS/2 Keyboard, PIT 8253, Serial COM1");
    println!("  Interrupts   : 8259 PIC Remapped + 256 IDT Descriptors");
    println!("  Shell        : Interactive command interpreter with line editing\n");
    vga_set_color(VgaColor::LightGrey, VgaColor::Black);
}

fn cmd_history(_args: &[&str]) {
    println!("\nCommand History: :3");
    println!("----------------------------------------------------");
    let hist = HISTORY.lock();
    let count = hist.count();
    if count == 0 {
        println!("  (no commands in history)");
    } else {
        for i in 0..count {
            if let Some(cmd) = hist.get(i) {
                println!("  {:2}  {}", i + 1, cmd);
            }
        }
    }
    println!("----------------------------------------------------");
}

pub fn shell_init() {
    vga_set_color(VgaColor::Yellow, VgaColor::Black);
    println!("Type 'help' to see available commands.\n");
    vga_set_color(VgaColor::LightGrey, VgaColor::Black);
}

fn shell_readline(buffer: &mut [u8]) -> usize {
    let mut index = 0;
    let max_len = buffer.len();
    let mut history_pos = HISTORY.lock().count();

    loop {
        let c = keyboard_getchar();

        if c == b'\n' || c == b'\r' {
            vga_putchar(b'\n');
            buffer[index] = 0;
            return index;
        } else if c == b'\x08' {
            if index > 0 {
                index -= 1;
                vga_putchar(b'\x08');
            }
        } else if c == KEY_UP {
            if history_pos > 0 {
                history_pos -= 1;
                while index > 0 {
                    vga_putchar(b'\x08');
                    index -= 1;
                }
                let hist = HISTORY.lock();
                if let Some(h) = hist.get(history_pos) {
                    let bytes = h.as_bytes();
                    for &b in bytes.iter().take(max_len - 1) {
                        buffer[index] = b;
                        index += 1;
                        vga_putchar(b);
                    }
                }
            }
        } else if c == KEY_DOWN {
            let count = HISTORY.lock().count();
            if history_pos + 1 < count {
                history_pos += 1;
                while index > 0 {
                    vga_putchar(b'\x08');
                    index -= 1;
                }
                let hist = HISTORY.lock();
                if let Some(h) = hist.get(history_pos) {
                    let bytes = h.as_bytes();
                    for &b in bytes.iter().take(max_len - 1) {
                        buffer[index] = b;
                        index += 1;
                        vga_putchar(b);
                    }
                }
            } else if history_pos < count {
                history_pos = count;
                while index > 0 {
                    vga_putchar(b'\x08');
                    index -= 1;
                }
            }
        } else if (32..=126).contains(&c) && index + 1 < max_len {
            buffer[index] = c;
            index += 1;
            vga_putchar(c);
        }
    }
}

pub fn shell_run() -> ! {
    let mut line_buf = [0u8; 256];

    loop {
        // Render colored shell prompt
        vga_set_color(VgaColor::LightCyan, VgaColor::Black);
        print!("pweos");
        vga_set_color(VgaColor::LightGreen, VgaColor::Black);
        print!("> ");
        vga_set_color(VgaColor::White, VgaColor::Black);

        let len = shell_readline(&mut line_buf);
        if len == 0 {
            continue;
        }

        let line_str = match core::str::from_utf8(&line_buf[..len]) {
            Ok(s) => s.trim(),
            Err(_) => continue,
        };

        if line_str.is_empty() {
            continue;
        }

        HISTORY.lock().add(line_str);

        let args: Vec<&str> = line_str.split_whitespace().collect();
        if args.is_empty() {
            continue;
        }

        let mut found = false;
        for cmd in COMMANDS {
            if cmd.name == args[0] {
                (cmd.handler)(&args);
                found = true;
                break;
            }
        }

        if !found {
            vga_set_color(VgaColor::LightRed, VgaColor::Black);
            println!(
                "Unknown command: '{}'. Type 'help' for command list.",
                args[0]
            );
            vga_set_color(VgaColor::LightGrey, VgaColor::Black);
        }
    }
}
