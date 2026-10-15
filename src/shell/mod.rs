extern crate alloc;
mod image;

use alloc::string::String;
use alloc::vec::Vec;
use crate::arch::cpu::cpu_get_info;
use crate::arch::io::{inb, outb};
use crate::drivers::keyboard::{keyboard_getchar, KEY_DELETE, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_UP};
use crate::drivers::rtc::rtc_get_datetime;
use crate::drivers::timer::{timer_get_ticks, timer_get_uptime_seconds, timer_sleep};
use crate::drivers::vga::{
    vga_begin_capture, vga_clear, vga_end_capture, vga_putchar, vga_set_color,
    vga_set_cursor_position, VgaColor, VGA_HEIGHT, VGA_WIDTH,
};
use crate::fs::ramfs::{
    ramfs_count_in, ramfs_create, ramfs_create_dir, ramfs_delete, ramfs_delete_dir, ramfs_is_dir,
    ramfs_list, ramfs_nuke, ramfs_read, ramfs_write,
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
static WORKING_DIRECTORY: Spinlock<String> = Spinlock::new(String::new());
static PIPE_INPUT: Spinlock<Vec<u8>> = Spinlock::new(Vec::new());
static PIPE_ACTIVE: Spinlock<bool> = Spinlock::new(false);

fn pipe_input() -> Vec<u8> {
    PIPE_INPUT.lock().clone()
}

fn resolve_path(path: &str) -> String {
    let mut resolved = if path.starts_with('/') {
        String::new()
    } else {
        WORKING_DIRECTORY.lock().clone()
    };
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if let Some(pos) = resolved.rfind('/') {
                    resolved.truncate(pos);
                } else {
                    resolved.clear();
                }
            }
            name => {
                if !resolved.is_empty() {
                    resolved.push('/');
                }
                resolved.push_str(name);
            }
        }
    }
    resolved
}

fn current_directory() -> String {
    WORKING_DIRECTORY.lock().clone()
}

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
        desc: "List files and directories in the RAM filesystem",
        handler: cmd_ls,
    },
    Command {
        name: "mkdir",
        desc: "Create a directory: mkdir <name>",
        handler: cmd_mkdir,
    },
    Command {
        name: "rmdir",
        desc: "Remove an empty directory: rmdir <path>",
        handler: cmd_rmdir,
    },
    Command {
        name: "nuke",
        desc: "Recursively remove a file or directory tree: nuke <path>",
        handler: cmd_nuke,
    },
    Command {
        name: "cd",
        desc: "Change directory: cd <path>",
        handler: cmd_cd,
    },
    Command {
        name: "pwd",
        desc: "Print the current working directory",
        handler: cmd_pwd,
    },
    Command {
        name: "cat",
        desc: "Display file or piped input: cat [filename]",
        handler: cmd_cat,
    },
    Command {
        name: "meow",
        desc: "Display the cat picture; press Escape to return",
        handler: cmd_meow,
    },
    Command {
        name: "grep",
        desc: "Search piped input or files: grep <text> [path...]",
        handler: cmd_grep,
    },
    Command {
        name: "touch",
        desc: "Create empty file: touch <filename>",
        handler: cmd_touch,
    },
    Command {
        name: "write",
        desc: "Write one line to a file: write <filename> <text>",
        handler: cmd_write,
    },
    Command {
        name: "edit",
        desc: "Edit a file or piped text: edit [path] (Escape saves)",
        handler: cmd_edit,
    },
    Command {
        name: "less",
        desc: "View a file or piped text: less [path] (q exits)",
        handler: cmd_less,
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

fn cmd_meow(_args: &[&str]) {
    println!("Meow! Press Escape to return to the shell.");
    image::show_cat();
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

fn cmd_ls(args: &[&str]) {
    let path = if args.len() > 1 { resolve_path(args[1]) } else { current_directory() };
    if !ramfs_is_dir(&path) {
        println!("ls: '{}' is not a directory.", args.get(1).unwrap_or(&"."));
        return;
    }
    println!("\n{} ({} items):", if path.is_empty() { "/" } else { path.as_str() }, ramfs_count_in(&path));
    ramfs_list(&path, |name, size, is_directory| {
        vga_set_color(VgaColor::LightCyan, VgaColor::Black);
        if is_directory {
            print!("  {:<16}", alloc::format!("{}/", name));
        } else {
            print!("  {:<16}", name);
        }
        vga_set_color(VgaColor::LightGrey, VgaColor::Black);
        if is_directory {
            println!(" <DIR>");
        } else {
            println!(" {} bytes", size);
        }
    });
    println!("----------------------------------------------------");
}

fn cmd_mkdir(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: mkdir <name>");
        return;
    }
    let path = resolve_path(args[1]);
    if ramfs_create_dir(&path) {
        println!("Created directory '{}/'.", path);
    } else {
        println!("Directory '{}' is invalid or already exists.", args[1]);
    }
}

fn cmd_rmdir(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: rmdir <path>");
        return;
    }
    let path = resolve_path(args[1]);
    if path == current_directory() {
        println!("rmdir: cannot remove the current working directory.");
        return;
    }
    if ramfs_delete_dir(&path) {
        println!("Removed directory '{}/'.", path);
    } else if ramfs_is_dir(&path) {
        println!("rmdir: '{}' is not empty.", args[1]);
    } else {
        println!("rmdir: '{}' is not a directory.", args[1]);
    }
}

fn cmd_nuke(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: nuke <path>");
        return;
    }
    let path = resolve_path(args[1]);
    let cwd = current_directory();
    let mut path_prefix = path.clone();
    path_prefix.push('/');
    if path.is_empty() || cwd == path || cwd.starts_with(&path_prefix) {
        println!("nuke: refusing to remove the filesystem root or current working directory.");
        return;
    }
    if ramfs_nuke(&path) {
        println!("Removed '{}' and everything inside it.", path);
    } else {
        println!("nuke: '{}' not found.", args[1]);
    }
}

fn cmd_cd(args: &[&str]) {
    let target = if args.len() < 2 { String::new() } else { resolve_path(args[1]) };
    if ramfs_is_dir(&target) {
        *WORKING_DIRECTORY.lock() = target;
    } else {
        println!("cd: '{}' is not a directory.", args.get(1).unwrap_or(&""));
    }
}

fn cmd_pwd(_args: &[&str]) {
    let path = current_directory();
    println!("/{}", path);
}

fn cmd_cat(args: &[&str]) {
    if args.len() < 2 {
        if *PIPE_ACTIVE.lock() {
            for byte in pipe_input() {
                vga_putchar(byte);
            }
        } else {
            println!("Usage: cat <filename>");
        }
        return;
    }
    let path = resolve_path(args[1]);
    match ramfs_read(&path) {
        Some(data) => {
            if *PIPE_ACTIVE.lock() {
                for &byte in &data {
                    vga_putchar(byte);
                }
                return;
            }
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

fn cmd_grep(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: grep <text> [path...]");
        return;
    }
    let needle = args[1].as_bytes();

    if args.len() > 2 {
        let multiple_files = args.len() > 3;
        for &arg in args.iter().skip(2) {
            let path = resolve_path(arg);
            match ramfs_read(&path) {
                Some(contents) => {
                    grep_matching_lines(&contents, needle, if multiple_files { Some(path.as_str()) } else { None });
                }
                None => println!("grep: '{}' not found.", arg),
            }
        }
    } else if *PIPE_ACTIVE.lock() {
        grep_matching_lines(&pipe_input(), needle, None);
    } else {
        println!("Usage: grep <text> [path...] (or provide piped input)");
    }
}

fn grep_matching_lines(input: &[u8], needle: &[u8], label: Option<&str>) {
    let mut line_start = 0;
    for index in 0..input.len() {
        if input[index] == b'\n' {
            let line = &input[line_start..=index];
            if needle.is_empty() || line.windows(needle.len()).any(|window| window == needle) {
                if let Some(label) = label {
                    print!("{}:", label);
                }
                for &byte in line {
                    vga_putchar(byte);
                }
            }
            line_start = index + 1;
        }
    }
    if line_start < input.len() {
        let line = &input[line_start..];
        if needle.is_empty() || line.windows(needle.len()).any(|window| window == needle) {
            if let Some(label) = label {
                print!("{}:", label);
            }
            for &byte in line {
                vga_putchar(byte);
            }
        }
    }
}

fn cmd_touch(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: touch <filename>");
        return;
    }
    let path = resolve_path(args[1]);
    if ramfs_create(&path, b"") {
        println!("Created file '{}'. :3", path);
    } else {
        println!("File '{}' already exists or creation failed.", args[1]);
    }
}

fn cmd_write(args: &[&str]) {
    if args.len() < 2 || (args.len() < 3 && !*PIPE_ACTIVE.lock()) {
        println!("Usage: write <filename> <text...>");
        return;
    }

    let content = if args.len() == 2 && *PIPE_ACTIVE.lock() {
        pipe_input()
    } else {
        let mut text = String::new();
        for (i, &arg) in args.iter().enumerate().skip(2) {
            if i > 2 {
                text.push(' ');
            }
            text.push_str(arg);
        }
        text.push('\n');
        text.into_bytes()
    };

    let len = content.len();
    let path = resolve_path(args[1]);
    if ramfs_write(&path, &content, false) {
        println!("Wrote {} bytes to '{}'. :3", len, path);
    } else {
        println!("Failed to write to '{}'. :(", args[1]);
    }
}

fn editor_line_bounds(contents: &[u8], line: usize) -> (usize, usize) {
    let mut current_line = 0;
    let mut start = 0;
    for (index, &byte) in contents.iter().enumerate() {
        if current_line == line {
            if byte == b'\n' {
                return (start, index);
            }
        } else if byte == b'\n' {
            current_line += 1;
            start = index + 1;
        }
    }
    if current_line == line {
        (start, contents.len())
    } else {
        (contents.len(), contents.len())
    }
}

fn editor_cursor_line_column(contents: &[u8], cursor: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    for &byte in contents.iter().take(cursor) {
        if byte == b'\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn move_text_cursor(contents: &[u8], cursor: &mut usize, preferred_column: &mut usize, key: u8) -> bool {
    match key {
        KEY_LEFT if *cursor > 0 => {
            *cursor -= 1;
            *preferred_column = editor_cursor_line_column(contents, *cursor).1;
            true
        }
        KEY_RIGHT if *cursor < contents.len() => {
            *cursor += 1;
            *preferred_column = editor_cursor_line_column(contents, *cursor).1;
            true
        }
        KEY_UP | KEY_DOWN => {
            let (line, _) = editor_cursor_line_column(contents, *cursor);
            let (_, line_end) = editor_line_bounds(contents, line);
            let target_line = if key == KEY_UP {
                line.checked_sub(1)
            } else if line_end < contents.len() {
                Some(line + 1)
            } else {
                None
            };
            if let Some(target_line) = target_line {
                let (target_start, target_end) = editor_line_bounds(contents, target_line);
                *cursor = target_start + (*preferred_column).min(target_end - target_start);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn editor_redraw(path: &str, contents: &[u8], cursor: usize, read_only: bool) {
    vga_clear();
    vga_set_color(VgaColor::LightCyan, VgaColor::Black);
    println!("PweOS {} — {}", if read_only { "viewer" } else { "editor" }, path);
    vga_set_color(VgaColor::LightGrey, VgaColor::Black);
    if read_only {
        println!("Arrows move through the file | q or Escape returns to shell.");
        println!("Read-only view.");
    } else {
        println!("Arrows move | Enter inserts a line | Backspace/Delete erase");
        println!("Escape saves and returns to the shell.");
    }
    println!("----------------------------------------------------");

    const HEADER_ROWS: usize = 4;
    let (cursor_line, cursor_column) = editor_cursor_line_column(contents, cursor);
    let view_rows = VGA_HEIGHT - HEADER_ROWS;
    let first_line = cursor_line.saturating_sub(view_rows - 1);
    let horizontal_offset = cursor_column.saturating_sub(VGA_WIDTH - 2);
    for row in 0..view_rows {
        let line = first_line + row;
        let (start, end) = editor_line_bounds(contents, line);
        let skip = if line == cursor_line { horizontal_offset } else { 0 };
        for &byte in contents[start..end].iter().skip(skip).take(VGA_WIDTH - 1) {
            if byte == b'\t' {
                vga_putchar(b' ');
            } else if (32..=126).contains(&byte) {
                vga_putchar(byte);
            } else {
                vga_putchar(b'.');
            }
        }
        if row + 1 < view_rows {
            vga_putchar(b'\n');
        }
    }
    let screen_row = HEADER_ROWS + cursor_line - first_line;
    let screen_column = cursor_column.saturating_sub(horizontal_offset);
    vga_set_cursor_position(screen_row, screen_column);
}

fn cmd_edit(args: &[&str]) {
    if args.len() < 2 && !*PIPE_ACTIVE.lock() {
        println!("Usage: edit <path>");
        return;
    }

    const EDITOR_MAX_BYTES: usize = 4096;
    let path = if args.len() >= 2 { Some(resolve_path(args[1])) } else { None };
    let mut contents = if *PIPE_ACTIVE.lock() {
        pipe_input()
    } else if let Some(path) = path.as_ref() {
        match ramfs_read(path) {
            Some(data) => data,
            None => {
                if !ramfs_create(path, b"") {
                    println!("edit: could not create '{}' (check the parent directory).", path);
                    return;
                }
                Vec::new()
            }
        }
    } else {
        pipe_input()
    };
    if contents.len() > EDITOR_MAX_BYTES {
        println!("edit: input exceeds the {} byte editor limit.", EDITOR_MAX_BYTES);
        return;
    }

    let mut cursor = contents.len();
    let (_, mut preferred_column) = editor_cursor_line_column(&contents, cursor);
    let display_path = path.as_deref().unwrap_or("<pipe>");
    editor_redraw(display_path, &contents, cursor, false);
    loop {
        let key = keyboard_getchar();
        if key == 27 {
            vga_set_color(VgaColor::LightGreen, VgaColor::Black);
            if let Some(path) = path.as_ref() {
                let saved = ramfs_write(path, &contents, false);
                println!("\n{} {} bytes to '{}'.", if saved { "Saved" } else { "Could not save" }, contents.len(), path);
            } else {
                print!("\nSave as (blank discards changes): ");
                let mut name_buf = [0u8; 128];
                let name_len = shell_readline(&mut name_buf);
                if name_len == 0 {
                    println!("Changes discarded.");
                } else if let Ok(name) = core::str::from_utf8(&name_buf[..name_len]) {
                    let save_path = resolve_path(name.trim());
                    let saved = ramfs_write(&save_path, &contents, false);
                    if saved {
                        println!("Saved {} bytes to '{}'.", contents.len(), save_path);
                    } else {
                        println!("Could not save '{}'.", save_path);
                    }
                } else {
                    println!("Invalid filename; changes discarded.");
                }
            }
            vga_set_color(VgaColor::LightGrey, VgaColor::Black);
            return;
        } else if move_text_cursor(&contents, &mut cursor, &mut preferred_column, key) {
            editor_redraw(display_path, &contents, cursor, false);
        } else if key == b'\x08' {
            if cursor > 0 {
                cursor -= 1;
                contents.remove(cursor);
                preferred_column = editor_cursor_line_column(&contents, cursor).1;
                editor_redraw(display_path, &contents, cursor, false);
            }
        } else if key == KEY_DELETE {
            if cursor < contents.len() {
                contents.remove(cursor);
                preferred_column = editor_cursor_line_column(&contents, cursor).1;
                editor_redraw(display_path, &contents, cursor, false);
            }
        } else if key == b'\n' || key == b'\r' {
            if contents.len() < EDITOR_MAX_BYTES {
                contents.insert(cursor, b'\n');
                cursor += 1;
                preferred_column = 0;
                editor_redraw(display_path, &contents, cursor, false);
            }
        } else if (32..=126).contains(&key) && contents.len() < EDITOR_MAX_BYTES {
            contents.insert(cursor, key);
            cursor += 1;
            preferred_column = editor_cursor_line_column(&contents, cursor).1;
            editor_redraw(display_path, &contents, cursor, false);
        }
    }
}

fn cmd_less(args: &[&str]) {
    if args.len() < 2 && !*PIPE_ACTIVE.lock() {
        println!("Usage: less <path>");
        return;
    }
    let (path, contents) = if args.len() >= 2 {
        let path = resolve_path(args[1]);
        let contents = match ramfs_read(&path) {
            Some(contents) => contents,
            None => {
                println!("less: '{}' not found.", args[1]);
                return;
            }
        };
        (path, contents)
    } else {
        (String::from("<pipe>"), pipe_input())
    };

    let mut cursor = 0;
    let mut preferred_column = 0;
    editor_redraw(&path, &contents, cursor, true);
    loop {
        let key = keyboard_getchar();
        if key == b'q' || key == 27 {
            vga_set_color(VgaColor::LightGrey, VgaColor::Black);
            println!("\nClosed '{}'.", path);
            return;
        }
        if move_text_cursor(&contents, &mut cursor, &mut preferred_column, key) {
            editor_redraw(&path, &contents, cursor, true);
        }
    }
}

fn cmd_rm(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: rm <filename>");
        return;
    }
    let path = resolve_path(args[1]);
    if ramfs_delete(&path) {
        println!("Deleted file '{}'. :3", path);
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

fn execute_pipeline(line: &str) -> bool {
    if !line.contains('|') {
        return false;
    }

    let mut input = Vec::new();
    let stages: Vec<&str> = line.split('|').collect();
    for stage in stages {
        let args: Vec<&str> = stage.split_whitespace().collect();
        if args.is_empty() {
            *PIPE_ACTIVE.lock() = false;
            *PIPE_INPUT.lock() = Vec::new();
            println!("Syntax error: missing command around '|'.");
            return true;
        }

        let command = COMMANDS.iter().find(|command| command.name == args[0]);
        if let Some(command) = command {
            *PIPE_INPUT.lock() = input;
            *PIPE_ACTIVE.lock() = true;
            if command.name == "less" || command.name == "edit" {
                (command.handler)(&args);
                input = Vec::new();
            } else {
                vga_begin_capture();
                (command.handler)(&args);
                input = vga_end_capture();
            }
        } else {
            *PIPE_ACTIVE.lock() = false;
            *PIPE_INPUT.lock() = Vec::new();
            vga_set_color(VgaColor::LightRed, VgaColor::Black);
            println!("Unknown command: '{}'. Type 'help' for command list.", args[0]);
            vga_set_color(VgaColor::LightGrey, VgaColor::Black);
            return true;
        }
    }

    *PIPE_ACTIVE.lock() = false;
    *PIPE_INPUT.lock() = Vec::new();
    vga_set_color(VgaColor::LightGrey, VgaColor::Black);
    for byte in input {
        vga_putchar(byte);
    }
    true
}

pub fn shell_run() -> ! {
    let mut line_buf = [0u8; 256];

    loop {
        // Render colored shell prompt
        vga_set_color(VgaColor::LightCyan, VgaColor::Black);
        print!("pweos");
        vga_set_color(VgaColor::LightGreen, VgaColor::Black);
        let cwd = current_directory();
        if cwd.is_empty() {
            print!("> ");
        } else {
            print!(":/{}> ", cwd);
        }
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

        if execute_pipeline(line_str) {
            continue;
        }

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
