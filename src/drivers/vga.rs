use alloc::vec::Vec;
use super::serial::{serial_init, serial_putchar};
use crate::arch::io::{inb, outb};
use crate::sync::Spinlock;
use core::fmt;

pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;
pub const VGA_BUFFER_ADDR: usize = 0xB8000;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum VgaColor {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

#[inline]
pub fn vga_entry_color(fg: VgaColor, bg: VgaColor) -> u8 {
    (fg as u8) | ((bg as u8) << 4)
}

#[inline]
pub fn vga_entry(uc: u8, color: u8) -> u16 {
    (uc as u16) | ((color as u16) << 8)
}

pub struct VgaWriter {
    buffer: *mut u16,
    pub col: usize,
    pub row: usize,
    pub current_color: u8,
    capture: Option<Vec<u8>>,
}

unsafe impl Send for VgaWriter {}

impl VgaWriter {
    pub const fn new() -> Self {
        Self {
            buffer: VGA_BUFFER_ADDR as *mut u16,
            col: 0,
            row: 0,
            current_color: 0x07, // Light grey on black
            capture: None,
        }
    }
}

impl Default for VgaWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl VgaWriter {

    pub fn init(&mut self) {
        serial_init();
        self.set_color(VgaColor::LightGrey, VgaColor::Black);
        self.clear();
    }

    pub fn set_color(&mut self, fg: VgaColor, bg: VgaColor) {
        self.current_color = vga_entry_color(fg, bg);
    }

    pub fn set_raw_color(&mut self, color: u8) {
        self.current_color = color;
    }

    pub fn get_color(&self) -> u8 {
        self.current_color
    }

    pub fn clear(&mut self) {
        let blank = vga_entry(b' ', self.current_color);
        for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
            unsafe {
                self.buffer.add(i).write_volatile(blank);
            }
        }
        self.col = 0;
        self.row = 0;
        self.update_hardware_cursor();
    }

    /// Write one text cell without mirroring the character to the serial port.
    pub fn write_cell(&mut self, col: usize, row: usize, character: u8, color: u8) {
        if col < VGA_WIDTH && row < VGA_HEIGHT {
            unsafe {
                self.buffer
                    .add(row * VGA_WIDTH + col)
                    .write_volatile(vga_entry(character, color));
            }
        }
    }

    pub fn update_hardware_cursor(&self) {
        let position = (self.row * VGA_WIDTH + self.col) as u16;
        unsafe {
            outb(0x3D4, 0x0F);
            outb(0x3D5, (position & 0xFF) as u8);
            outb(0x3D4, 0x0E);
            outb(0x3D5, ((position >> 8) & 0xFF) as u8);
        }
    }

    pub fn set_cursor_position(&mut self, row: usize, col: usize) {
        self.row = row.min(VGA_HEIGHT - 1);
        self.col = col.min(VGA_WIDTH - 1);
        self.update_hardware_cursor();
    }

    pub fn scroll(&mut self) {
        for y in 0..(VGA_HEIGHT - 1) {
            for x in 0..VGA_WIDTH {
                unsafe {
                    let next_val = self.buffer.add((y + 1) * VGA_WIDTH + x).read_volatile();
                    self.buffer.add(y * VGA_WIDTH + x).write_volatile(next_val);
                }
            }
        }

        // Clear bottom row
        let blank = vga_entry(b' ', self.current_color);
        for x in 0..VGA_WIDTH {
            unsafe {
                self.buffer
                    .add((VGA_HEIGHT - 1) * VGA_WIDTH + x)
                    .write_volatile(blank);
            }
        }

        self.row = VGA_HEIGHT - 1;
    }

    pub fn putchar(&mut self, c: u8) {
        if let Some(output) = self.capture.as_mut() {
            output.push(c);
            return;
        }

        // Send to serial port
        if c == b'\n' {
            serial_putchar(b'\r');
        }
        serial_putchar(c);

        // Handle control characters
        if c == b'\n' {
            self.col = 0;
            self.row += 1;
        } else if c == b'\r' {
            self.col = 0;
        } else if c == b'\t' {
            let tab_stop = (self.col + 4) & !3;
            if tab_stop >= VGA_WIDTH {
                self.col = 0;
                self.row += 1;
            } else {
                self.col = tab_stop;
            }
        } else if c == b'\x08' {
            // Backspace
            if self.col > 0 {
                self.col -= 1;
                unsafe {
                    self.buffer
                        .add(self.row * VGA_WIDTH + self.col)
                        .write_volatile(vga_entry(b' ', self.current_color));
                }
            }
        } else {
            unsafe {
                self.buffer
                    .add(self.row * VGA_WIDTH + self.col)
                    .write_volatile(vga_entry(c, self.current_color));
            }
            self.col += 1;
            if self.col >= VGA_WIDTH {
                self.col = 0;
                self.row += 1;
            }
        }

        if self.row >= VGA_HEIGHT {
            self.scroll();
        }

        self.update_hardware_cursor();
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.putchar(b);
        }
    }

    pub fn begin_capture(&mut self) {
        self.capture = Some(Vec::new());
    }

    pub fn end_capture(&mut self) -> Vec<u8> {
        self.capture.take().unwrap_or_default()
    }

    pub fn set_cursor(&mut self, col: usize, row: usize) {
        self.col = if col >= VGA_WIDTH { VGA_WIDTH - 1 } else { col };
        self.row = if row >= VGA_HEIGHT {
            VGA_HEIGHT - 1
        } else {
            row
        };
        self.update_hardware_cursor();
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.col, self.row)
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}

pub static VGA_WRITER: Spinlock<VgaWriter> = Spinlock::new(VgaWriter::new());

pub fn vga_init() {
    VGA_WRITER.lock().init();
}

pub fn vga_clear() {
    VGA_WRITER.lock().clear();
}

pub fn vga_set_color(fg: VgaColor, bg: VgaColor) {
    VGA_WRITER.lock().set_color(fg, bg);
}

pub fn vga_get_color() -> u8 {
    VGA_WRITER.lock().get_color()
}

pub fn vga_set_raw_color(color: u8) {
    VGA_WRITER.lock().set_raw_color(color);
}

/// Set VGA text blink behavior and return the previous attribute mode value.
pub fn vga_set_blink_enabled(enabled: bool) -> u8 {
    let _ = unsafe { inb(0x3DA) };
    unsafe { outb(0x3C0, 0x30 | 0x10) };
    let previous = unsafe { inb(0x3C1) };
    let _ = unsafe { inb(0x3DA) };
    unsafe {
        outb(0x3C0, 0x30 | 0x10);
        outb(
            0x3C0,
            if enabled {
                previous | 0x08
            } else {
                previous & !0x08
            },
        );
    }
    previous
}

/// Set the 16 logical text colors and return their previous DAC colors.
pub fn vga_set_text_palette(colors: &[[u8; 3]; 16]) -> [[u8; 3]; 16] {
    const DAC_INDEX: [u8; 16] = [0, 1, 2, 3, 4, 5, 20, 7, 56, 57, 58, 59, 60, 61, 62, 63];
    let mut previous = [[0u8; 3]; 16];
    for (i, index) in DAC_INDEX.iter().enumerate() {
        unsafe {
            outb(0x3C7, *index);
            previous[i] = [
                inb(0x3C9) << 2,
                inb(0x3C9) << 2,
                inb(0x3C9) << 2,
            ];
            outb(0x3C8, *index);
            outb(0x3C9, colors[i][0] >> 2);
            outb(0x3C9, colors[i][1] >> 2);
            outb(0x3C9, colors[i][2] >> 2);
        }
    }
    previous
}

pub fn vga_restore_text_palette(colors: &[[u8; 3]; 16]) {
    const DAC_INDEX: [u8; 16] = [0, 1, 2, 3, 4, 5, 20, 7, 56, 57, 58, 59, 60, 61, 62, 63];
    for (i, index) in DAC_INDEX.iter().enumerate() {
        unsafe {
            outb(0x3C8, *index);
            outb(0x3C9, colors[i][0] >> 2);
            outb(0x3C9, colors[i][1] >> 2);
            outb(0x3C9, colors[i][2] >> 2);
        }
    }
}

pub fn vga_restore_attribute_mode(mode: u8) {
    let _ = unsafe { inb(0x3DA) };
    unsafe {
        outb(0x3C0, 0x30 | 0x10);
        outb(0x3C0, mode);
    }
}

pub fn vga_putchar(c: u8) {
    VGA_WRITER.lock().putchar(c);
}

pub fn vga_set_cursor_position(row: usize, col: usize) {
    VGA_WRITER.lock().set_cursor_position(row, col);
}

pub fn vga_begin_capture() {
    VGA_WRITER.lock().begin_capture();
}

pub fn vga_end_capture() -> Vec<u8> {
    VGA_WRITER.lock().end_capture()
}

pub fn vga_puts(str: &str) {
    VGA_WRITER.lock().write_bytes(str.as_bytes());
}

pub fn vga_set_cursor(col: usize, row: usize) {
    VGA_WRITER.lock().set_cursor(col, row);
}

pub fn vga_get_cursor() -> (usize, usize) {
    VGA_WRITER.lock().get_cursor()
}
