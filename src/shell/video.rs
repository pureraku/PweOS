use alloc::vec;
use alloc::vec::Vec;

use crate::drivers::ata::read_sectors;
use crate::drivers::framebuffer::{Framebuffer, HEIGHT, WIDTH};
use crate::drivers::keyboard::{keyboard_getchar, keyboard_has_char};
use crate::drivers::timer::timer_sleep;
use crate::drivers::vga::vga_clear;
use crate::fs::ramfs::ramfs_video_metadata;

const PIXELS: usize = WIDTH * HEIGHT;

/// Load a bundled video from the companion IDE disk. `Ok(None)` means the
/// requested path is not one of the videos included at build time.
pub fn read_bundled(path: &str) -> Result<Option<Vec<u8>>, &'static str> {
    let Some(name) = path.strip_prefix("videos/") else { return Ok(None) };
    let Some((start_sector, byte_len)) = ramfs_video_metadata(name) else { return Ok(None) };
    if byte_len == 0 || byte_len > 1_500_000 {
        return Err("video is too large for the current kernel heap");
    }
    let sectors = byte_len.div_ceil(512);
    let mut data = vec![0u8; sectors * 512];
    read_sectors(start_sector, sectors, &mut data)?;
    data.truncate(byte_len);
    Ok(Some(data))
}

/// Play a compact PWEV v1 video from RAMFS. Frames are RGB332 keyframes or
/// sparse changed-pixel deltas, with periodic timer-paced playback.
pub fn play(data: &[u8]) -> Result<(), &'static str> {
    if data.len() < 12 || &data[..4] != b"PWEV" || data[4] != 1 {
        return Err("unsupported or invalid video header");
    }
    let fps = data[5] as u32;
    let frame_count = read_u16(data, 6)? as usize;
    let width = read_u16(data, 8)? as usize;
    let height = read_u16(data, 10)? as usize;
    if width != WIDTH || height != HEIGHT || fps == 0 || fps > 30 || frame_count == 0 {
        return Err("video must be 320x200, 1-30 FPS, and contain frames");
    }

    let mut framebuffer = Framebuffer::new();
    let mut pixels = vec![0u8; PIXELS];
    let mut cursor = 12usize;
    let mut has_keyframe = false;
    let mut cancelled = false;
    let mut failure = None;

    for _ in 0..frame_count {
        let frame_kind = match data.get(cursor) {
            Some(&kind) => kind,
            None => { failure = Some("truncated video frame"); break; }
        };
        cursor += 1;
        let length = match read_u32(data, cursor) {
            Ok(length) => length as usize,
            Err(error) => { failure = Some(error); break; }
        };
        cursor += 4;
        let end = match cursor.checked_add(length) {
            Some(end) if end <= data.len() => end,
            _ => { failure = Some("truncated video frame data"); break; }
        };
        let result = match frame_kind {
            0 => decode_keyframe(&data[cursor..end], &mut pixels),
            1 if has_keyframe => decode_delta(&data[cursor..end], &mut pixels),
            1 => Err("video starts with a delta frame"),
            _ => Err("unknown video frame type"),
        };
        if let Err(error) = result {
            failure = Some(error);
            break;
        }
        has_keyframe = true;
        cursor = end;
        let _ = framebuffer.blit_rgb332(&pixels);

        if keyboard_has_char() {
            let _ = keyboard_getchar();
            cancelled = true;
            break;
        }
        timer_sleep((100 / fps).max(1));
    }

    if failure.is_none() && !cancelled && cursor != data.len() {
        failure = Some("unexpected bytes after video frames");
    }
    framebuffer.leave();
    vga_clear();
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn decode_keyframe(encoded: &[u8], pixels: &mut [u8]) -> Result<(), &'static str> {
    if encoded.len() % 2 != 0 {
        return Err("invalid video keyframe encoding");
    }
    let mut output = 0usize;
    for pair in encoded.chunks_exact(2) {
        let count = pair[0] as usize;
        if count == 0 || output + count > PIXELS {
            return Err("invalid video keyframe run");
        }
        pixels[output..output + count].fill(pair[1]);
        output += count;
    }
    if output != PIXELS {
        return Err("incomplete video keyframe");
    }
    Ok(())
}

fn decode_delta(encoded: &[u8], pixels: &mut [u8]) -> Result<(), &'static str> {
    let mut input = 0usize;
    let mut output = 0usize;
    while output < PIXELS {
        if input + 4 > encoded.len() {
            return Err("truncated video delta");
        }
        let skip = u16::from_le_bytes([encoded[input], encoded[input + 1]]) as usize;
        let run = u16::from_le_bytes([encoded[input + 2], encoded[input + 3]]) as usize;
        input += 4;
        output = output.checked_add(skip).ok_or("video delta overflow")?;
        if output > PIXELS || output + run > PIXELS || input + run > encoded.len() {
            return Err("invalid video delta run");
        }
        if run == 0 {
            if output != PIXELS || input != encoded.len() {
                return Err("invalid video delta terminator");
            }
            break;
        }
        pixels[output..output + run].copy_from_slice(&encoded[input..input + run]);
        input += run;
        output += run;
    }
    if output != PIXELS || input != encoded.len() {
        return Err("incomplete video delta");
    }
    Ok(())
}

fn read_u16(data: &[u8], at: usize) -> Result<u16, &'static str> {
    let bytes = data.get(at..at + 2).ok_or("truncated video header")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], at: usize) -> Result<u32, &'static str> {
    let bytes = data.get(at..at + 4).ok_or("truncated video frame header")?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
