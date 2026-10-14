use super::io::{inb, io_wait, outb};

pub const PIC1_COMMAND: u16 = 0x20;
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_COMMAND: u16 = 0xA0;
pub const PIC2_DATA: u16 = 0xA1;

pub const PIC_EOI: u8 = 0x20;

const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

pub unsafe fn pic_remap(offset1: u8, offset2: u8) {
    let mask1 = inb(PIC1_DATA);
    let mask2 = inb(PIC2_DATA);

    // Starts initialization sequence (in cascade mode)
    outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_wait();
    outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_wait();

    // ICW2: Vector offsets
    outb(PIC1_DATA, offset1);
    io_wait();
    outb(PIC2_DATA, offset2);
    io_wait();

    // ICW3: Cascade identity
    outb(PIC1_DATA, 4); // Tell Master PIC that there is a slave PIC at IRQ2 (0000 0100)
    io_wait();
    outb(PIC2_DATA, 2); // Tell Slave PIC its cascade identity (0000 0010)
    io_wait();

    // ICW4: Set 8086 mode
    outb(PIC1_DATA, ICW4_8086);
    io_wait();
    outb(PIC2_DATA, ICW4_8086);
    io_wait();

    // Restore masks
    outb(PIC1_DATA, mask1);
    outb(PIC2_DATA, mask2);
}

pub unsafe fn pic_send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, PIC_EOI);
    }
    outb(PIC1_COMMAND, PIC_EOI);
}

pub unsafe fn pic_set_mask(mut irq_line: u8) {
    let port = if irq_line < 8 {
        PIC1_DATA
    } else {
        irq_line -= 8;
        PIC2_DATA
    };
    let value = inb(port) | (1 << irq_line);
    outb(port, value);
}

pub unsafe fn pic_clear_mask(mut irq_line: u8) {
    let port = if irq_line < 8 {
        PIC1_DATA
    } else {
        irq_line -= 8;
        PIC2_DATA
    };
    let value = inb(port) & !(1 << irq_line);
    outb(port, value);
}
