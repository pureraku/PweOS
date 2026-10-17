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
use fs::ramfs::{ramfs_count, ramfs_init};
use mm::heap::{heap_init, HEAP_INITIAL_SIZE, HEAP_START_ADDR};
use shell::{shell_init, shell_run};

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

    // Enable CPU hardware interrupts
    unsafe {
        core::arch::asm!("sti");
    }

    vga_set_color(VgaColor::LightCyan, VgaColor::Black);
    println!("================================================================================");
    vga_set_color(VgaColor::LightGreen, VgaColor::Black);
    println!("                      Welcome to PweOS (32-bit x86)                             ");
    vga_set_color(VgaColor::LightCyan, VgaColor::Black);
    println!("================================================================================\n");

    vga_set_color(VgaColor::White, VgaColor::Black);
    println!("[+] CPU: Intel 80386+ in 32-bit Protected Mode (IF=1 interrupts active)");
    println!("[+] Memory: Flat model active (base: 0x0, limit: 4GB)");
    println!(
        "[+] Heap: Dynamic allocator initialized at 0x{:x} (size: {} KB)",
        HEAP_START_ADDR,
        HEAP_INITIAL_SIZE / 1024
    );
    println!(
        "[+] RAMFS: In-memory filesystem initialized ({} default files mounted)",
        ramfs_count()
    );
    println!("[+] IDT: 256 descriptor gates loaded, PIC remapped to vectors 0x20..0x2F");
    println!("[+] PIT: Timer running at 100 Hz (IRQ0 active)");
    println!("[+] Keyboard: PS/2 controller driver online (IRQ1 active)");
    println!(
        "[+] Console: VGA {}x{} text buffer at 0xb8000",
        VGA_WIDTH, VGA_HEIGHT
    );
    println!("[+] Serial: COM1 active at port 0x3f8 (38400 baud, 8N1)\n");
    println!("[+] Video storage: ATA primary-master PIO data disk\n");

    shell_init();
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
