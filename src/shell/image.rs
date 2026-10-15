use crate::drivers::keyboard::keyboard_wait_for_keypress;
use crate::drivers::framebuffer::{Framebuffer, HEIGHT as FRAMEBUFFER_HEIGHT, WIDTH as FRAMEBUFFER_WIDTH};
use crate::drivers::vga::vga_clear;

/// Draw a raw RGB332 image through the shared VGA framebuffer API.
/// Returns false if the image doesn't match the supported dimensions.
pub fn show_rgb332(pixels: &[u8], width: usize, height: usize) -> bool {
    if width != FRAMEBUFFER_WIDTH
        || height != FRAMEBUFFER_HEIGHT
        || pixels.len() != width * height
    {
        return false;
    }

    let mut framebuffer = Framebuffer::new();
    for (y, row) in pixels.chunks_exact(width).enumerate() {
        for (x, &color) in row.iter().enumerate() {
            framebuffer.pixel(x, y, color);
        }
    }

    keyboard_wait_for_keypress();
    framebuffer.leave();
    vga_clear();
    true
}
