#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

#[macro_use]
pub mod print;
pub mod sync;
pub mod arch;
pub mod drivers;
pub mod fs;
pub mod mm;
pub mod process;
pub mod shell;

use arch::gdt::gdt_init;
use arch::isr::isr_init;
use arch::paging::paging_init;
use core::panic::PanicInfo;
use drivers::keyboard::keyboard_init;
use drivers::timer::timer_init;
use drivers::vga::{vga_init, vga_set_color, VgaColor, VGA_HEIGHT, VGA_WIDTH};
use fs::ramfs::ramfs_init;
use mm::heap::{heap_init, HEAP_INITIAL_SIZE, HEAP_START_ADDR};
use shell::{shell_run, PWEOS_VERSION};

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    vga_init();
    gdt_init();
    paging_init();
    isr_init();
    heap_init(HEAP_START_ADDR, HEAP_INITIAL_SIZE);
    ramfs_init();
    timer_init(100);
    keyboard_init();
    let network_available = drivers::network::network_init();

    // Enable CPU hardware interrupts
    unsafe {
        core::arch::asm!("sti");
    }

    vga_set_color(VgaColor::LightCyan, VgaColor::Black);
    println!("       ____              pweos@pweos");
    println!("      |  _ \\             ----------------");
    println!("      | |_) |            OS: PweOS {}", PWEOS_VERSION);
    println!("      |  __/             Arch: i686");
    println!("      |_|                Kernel: Rust (no_std)");
    println!("                         Shell: PweOS");
    println!("                         Display: VGA {}x{}", VGA_WIDTH, VGA_HEIGHT);
    println!("                         Memory: {} KB heap", HEAP_INITIAL_SIZE / 1024);
    println!("                         Filesystem: RAMFS");
    println!("                         Network: {}", if network_available { "RTL8139" } else { "unavailable" });
    println!("                         Compiler: TinyCC (i386)");
    println!("                         CPU mode: 32-bit protected mode\n");

    vga_set_color(VgaColor::LightGrey, VgaColor::Black);
    println!("Type 'help' to see available commands.\n");
    vga_set_color(VgaColor::White, VgaColor::Black);

    shell_run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_set_color(VgaColor::White, VgaColor::Red);
    println!("\n*** KERNEL RUST PANIC ***");
    println!("{}", info);
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}
