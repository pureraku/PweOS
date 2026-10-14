use crate::arch::io::{inb, outb};

pub const COM1_PORT: u16 = 0x3F8;
pub const COM1_LSR_PORT: u16 = COM1_PORT + 5; // Line Status Register: 0x3FD

pub fn serial_init() {
    unsafe {
        outb(COM1_PORT + 1, 0x00); // Disable interrupts
        outb(COM1_PORT + 3, 0x80); // Enable DLAB (set baud rate divisor)
        outb(COM1_PORT, 0x03); // Divisor 3 (lo byte) -> 38400 baud
        outb(COM1_PORT + 1, 0x00); //           (hi byte)
        outb(COM1_PORT + 3, 0x03); // 8 bits, no parity, 1 stop bit
        outb(COM1_PORT + 2, 0xC7); // Enable FIFO, clear, 14-byte threshold
        outb(COM1_PORT + 4, 0x0B); // IRQs enabled, RTS/DSR set
    }
}

pub fn serial_putchar(c: u8) {
    unsafe {
        while (inb(COM1_LSR_PORT) & 0x20) == 0 {}
        outb(COM1_PORT, c);
    }
}

pub fn serial_has_byte() -> bool {
    unsafe { (inb(COM1_LSR_PORT) & 0x01) != 0 }
}

pub fn serial_getchar() -> u8 {
    unsafe {
        while !serial_has_byte() {}
        inb(COM1_PORT)
    }
}
