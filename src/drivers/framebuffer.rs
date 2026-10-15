//! Minimal pixel drawing API for the VGA 320x200, 256-color mode.

use core::ptr::write_volatile;

use super::vga::{vga_enter_rgb332_mode, vga_restore_text_mode};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 200;
const VGA_MEMORY: usize = 0xA0000;

/// RGB332 color: 3 red bits, 3 green bits, and 2 blue bits.
pub const fn rgb332(red: u8, green: u8, blue: u8) -> u8 {
    (red & 0xE0) | ((green >> 3) & 0x1C) | (blue >> 6)
}

/// A framebuffer view backed by VGA mode 13h's 64,000-byte pixel memory.
pub struct Framebuffer;

impl Framebuffer {
    pub fn new() -> Self {
        vga_enter_rgb332_mode();
        Self
    }

    pub fn clear(&mut self, color: u8) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                self.pixel(x, y, color);
            }
        }
    }

    /// Copy a complete packed RGB332 frame into the framebuffer.
    pub fn blit_rgb332(&mut self, pixels: &[u8]) -> bool {
        if pixels.len() != WIDTH * HEIGHT {
            return false;
        }
        for (offset, &color) in pixels.iter().enumerate() {
            unsafe { write_volatile((VGA_MEMORY as *mut u8).add(offset), color) };
        }
        true
    }

    pub fn pixel(&mut self, x: usize, y: usize, color: u8) {
        if x < WIDTH && y < HEIGHT {
            unsafe { write_volatile((VGA_MEMORY as *mut u8).add(y * WIDTH + x), color) };
        }
    }

    /// Draw a clipped line with the integer Bresenham algorithm.
    pub fn line(&mut self, start: (i32, i32), end: (i32, i32), color: u8) {
        let (mut x0, mut y0) = start;
        let (x1, y1) = end;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.pixel(x0 as usize, y0 as usize, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice_error = 2 * error;
            if twice_error >= dy {
                error += dy;
                x0 += sx;
            }
            if twice_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    /// Fill a triangle using integer barycentric edge tests.
    pub fn fill_triangle(&mut self, points: [(i32, i32); 3], color: u8) {
        let points = points.map(|(x, y)| {
            (x.clamp(0, (WIDTH - 1) as i32), y.clamp(0, (HEIGHT - 1) as i32))
        });
        let [(ax, ay), (bx, by), (cx, cy)] = points;
        let min_x = ax.min(bx).min(cx).max(0) as usize;
        let max_x = ax.max(bx).max(cx).min((WIDTH - 1) as i32) as usize;
        let min_y = ay.min(by).min(cy).max(0) as usize;
        let max_y = ay.max(by).max(cy).min((HEIGHT - 1) as i32) as usize;
        let area = edge(ax, ay, bx, by, cx, cy);
        if area == 0 || min_x > max_x || min_y > max_y {
            return;
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as i32;
                let py = y as i32;
                let e0 = edge(ax, ay, bx, by, px, py);
                let e1 = edge(bx, by, cx, cy, px, py);
                let e2 = edge(cx, cy, ax, ay, px, py);
                if (e0 >= 0 && e1 >= 0 && e2 >= 0) || (e0 <= 0 && e1 <= 0 && e2 <= 0) {
                    self.pixel(x, y, color);
                }
            }
        }
    }

    /// Restore the saved text-mode registers and palette.
    pub fn leave(self) {
        vga_restore_text_mode();
    }
}

#[inline]
fn edge(ax: i32, ay: i32, bx: i32, by: i32, px: i32, py: i32) -> i32 {
    (px - ax) * (by - ay) - (py - ay) * (bx - ax)
}
