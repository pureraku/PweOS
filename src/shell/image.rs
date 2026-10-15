use crate::drivers::keyboard::keyboard_wait_for_keypress;
use crate::drivers::vga::{vga_enter_rgb332_mode, vga_restore_text_mode};
use core::ptr::write_volatile;

const IMAGE_WIDTH: usize = 320;
const IMAGE_HEIGHT: usize = 200;
const VGA_FRAMEBUFFER: usize = 0xA0000;

/// Draw a raw RGB332 image using the VGA's 320x200, 256-color mode.
/// Returns false if the image doesn't match the supported dimensions.
pub fn show_rgb332(pixels: &[u8], width: usize, height: usize) -> bool {
    if width != IMAGE_WIDTH || height != IMAGE_HEIGHT || pixels.len() != width * height {
        return false;
    }

    vga_enter_rgb332_mode();
    unsafe {
        let framebuffer = VGA_FRAMEBUFFER as *mut u8;
        for (offset, &pixel) in pixels.iter().enumerate() {
            write_volatile(framebuffer.add(offset), pixel);
        }
    }

    keyboard_wait_for_keypress();
    vga_restore_text_mode();
    true
}
