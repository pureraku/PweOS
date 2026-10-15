use crate::drivers::keyboard::keyboard_wait_for_keypress;
use crate::drivers::vga::{
    vga_clear, vga_restore_attribute_mode, vga_restore_text_palette, vga_set_blink_enabled,
    vga_set_text_palette, VGA_HEIGHT, VGA_WIDTH, VGA_WRITER,
};

const CAT_WIDTH: usize = 320;
const CAT_HEIGHT: usize = 200;
const HALF_BLOCK: u8 = 0xDF;

const CAT_PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [23, 35, 52],
    [16, 6, 6],
    [33, 17, 18],
    [43, 26, 22],
    [55, 37, 28],
    [66, 48, 37],
    [79, 61, 50],
    [93, 75, 60],
    [66, 87, 120],
    [117, 104, 97],
    [143, 125, 106],
    [161, 143, 124],
    [184, 167, 144],
    [222, 208, 184],
    [255, 248, 232],
];

pub fn show_cat() {
    let pixels = include_bytes!("../../assets/cat.rgb332");
    show_rgb332(pixels, CAT_WIDTH, CAT_HEIGHT);
}

fn show_rgb332(pixels: &[u8], width: usize, height: usize) {
    let output_width = VGA_WIDTH;
    let output_height = VGA_HEIGHT * 2;
    if width == 0
        || height == 0
        || pixels.len() != width * height
        || width % output_width != 0
        || height % output_height != 0
    {
        return;
    }

    let old_attribute_mode = vga_set_blink_enabled(false);
    let old_palette = vga_set_text_palette(&CAT_PALETTE);
    draw_rgb332(pixels, width, output_width, output_height);
    keyboard_wait_for_keypress();
    vga_clear();
    vga_restore_text_palette(&old_palette);
    vga_restore_attribute_mode(old_attribute_mode);
}

fn draw_rgb332(pixels: &[u8], width: usize, output_width: usize, output_height: usize) {
    let sample_width = width / output_width;
    let sample_height = pixels.len() / width / output_height;
    let mut writer = VGA_WRITER.lock();

    for row in 0..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            let foreground = average_color(
                pixels,
                width,
                col * sample_width,
                row * 2 * sample_height,
                sample_width,
                sample_height,
            );
            let background = average_color(
                pixels,
                width,
                col * sample_width,
                (row * 2 + 1) * sample_height,
                sample_width,
                sample_height,
            );
            writer.write_cell(col, row, HALF_BLOCK, foreground | (background << 4));
        }
    }
}

fn average_color(
    pixels: &[u8],
    width: usize,
    start_x: usize,
    start_y: usize,
    sample_width: usize,
    sample_height: usize,
) -> u8 {
    let mut sums = [0u32; 3];
    let sample_count = (sample_width * sample_height) as u32;
    for y in start_y..start_y + sample_height {
        for x in start_x..start_x + sample_width {
            let pixel = pixels[y * width + x];
            sums[0] += (((pixel >> 5) & 0x07) as u32 * 255) / 7;
            sums[1] += (((pixel >> 2) & 0x07) as u32 * 255) / 7;
            sums[2] += ((pixel & 0x03) as u32 * 255) / 3;
        }
    }

    nearest_vga_color(
        (sums[0] / sample_count) as i32,
        (sums[1] / sample_count) as i32,
        (sums[2] / sample_count) as i32,
    )
}

fn nearest_vga_color(red: i32, green: i32, blue: i32) -> u8 {
    let mut nearest = 0;
    let mut best_distance = i32::MAX;
    for (index, &[r, g, b]) in CAT_PALETTE.iter().enumerate() {
        let (r, g, b) = (r as i32, g as i32, b as i32);
        let dr = red - r;
        let dg = green - g;
        let db = blue - b;
        let distance = dr * dr + dg * dg + db * db;
        if distance < best_distance {
            nearest = index as u8;
            best_distance = distance;
        }
    }
    nearest
}
