//! ATAPI CD-ROM sector reader.
//!
//! Probes for an ATAPI device on the secondary ATA bus (0x170) or primary
//! slave (0x1F0/0xB0), then issues SCSI READ(10) packets to read 2048-byte
//! CD-ROM sectors.

use crate::arch::io::{inb, inw, io_wait, outb, outw};
use crate::sync::Spinlock;

pub const CD_SECTOR_SIZE: usize = 2048;

const ATA_CMD_PACKET: u8 = 0xA0;
const ATA_CMD_ID_PACKET: u8 = 0xA1;
const SCSI_READ_10: u8 = 0x28;

const STATUS_BUSY: u8 = 0x80;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

/// Cached CD-ROM drive location: (base_port, drive_select_byte)
static CD_DRIVE: Spinlock<Option<(u16, u8)>> = Spinlock::new(None);

/// Return the (base_port, drive_sel) for the first ATAPI CD-ROM found, caching the result.
fn get_drive() -> Option<(u16, u8)> {
    {
        let cached = CD_DRIVE.lock();
        if let Some(drive) = *cached {
            return Some(drive);
        }
    }
    // Try: secondary master (0x170,0xA0), secondary slave (0x170,0xB0), primary slave (0x1F0,0xB0)
    let candidates: &[(u16, u8)] = &[
        (0x170, 0xA0),
        (0x170, 0xB0),
        (0x1F0, 0xB0),
    ];
    for &(base, sel) in candidates {
        if probe_atapi(base, sel) {
            *CD_DRIVE.lock() = Some((base, sel));
            return Some((base, sel));
        }
    }
    None
}

fn probe_atapi(base: u16, drive_sel: u8) -> bool {
    unsafe {
        outb(base + 6, drive_sel);
        io_wait(); io_wait(); io_wait(); io_wait();
        let status = inb(base + 7);
        if status == 0xFF || status == 0x00 {
            return false;
        }
        outb(base + 7, ATA_CMD_ID_PACKET);
        io_wait();
        let status = inb(base + 7);
        if status == 0x00 {
            return false;
        }
        for _ in 0..100_000usize {
            let s = inb(base + 7);
            if s & STATUS_BUSY == 0 {
                if s & STATUS_DRQ != 0 {
                    // Drain 256 words of identify data
                    for _ in 0..256 {
                        let _ = inw(base);
                    }
                    return true;
                }
                return false;
            }
            core::hint::spin_loop();
        }
        false
    }
}

fn wait_not_busy(base: u16) -> Result<(), &'static str> {
    for _ in 0..10_000_000usize {
        let s = unsafe { inb(base + 7) };
        if s == 0xFF {
            return Err("ATAPI: drive disappeared");
        }
        if s & STATUS_BUSY == 0 {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("ATAPI: timeout waiting for not-busy")
}

fn wait_drq(base: u16) -> Result<(), &'static str> {
    for _ in 0..10_000_000usize {
        let s = unsafe { inb(base + 7) };
        if s & STATUS_ERR != 0 {
            return Err("ATAPI: device error");
        }
        if s & STATUS_BUSY == 0 && s & STATUS_DRQ != 0 {
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("ATAPI: timeout waiting for DRQ")
}

/// Read one 2048-byte CD sector at `lba` into `sector` (must be exactly 2048 bytes).
fn read_one(base: u16, drive_sel: u8, lba: u32, sector: &mut [u8]) -> Result<(), &'static str> {
    debug_assert_eq!(sector.len(), CD_SECTOR_SIZE);
    wait_not_busy(base)?;
    unsafe {
        outb(base + 6, drive_sel);
        io_wait();
        outb(base + 1, 0x00);                            // no DMA
        outb(base + 4, (CD_SECTOR_SIZE & 0xFF) as u8);   // byte count low
        outb(base + 5, (CD_SECTOR_SIZE >> 8) as u8);     // byte count high
        outb(base + 7, ATA_CMD_PACKET);                  // PACKET command
        io_wait();
    }
    wait_drq(base)?;
    // Send 12-byte SCSI READ(10) packet as 6 x 16-bit words
    let packet: [u8; 12] = [
        SCSI_READ_10, 0x00,
        (lba >> 24) as u8, (lba >> 16) as u8, (lba >> 8) as u8, lba as u8,
        0x00,
        0x00, 0x01, // transfer length = 1 sector
        0x00, 0x00, 0x00,
    ];
    for chunk in packet.chunks(2) {
        let word = (chunk[0] as u16) | ((chunk[1] as u16) << 8);
        unsafe { outw(base, word); }
    }
    wait_drq(base)?;
    // Read 1024 words = 2048 bytes
    for i in 0..1024usize {
        let word = unsafe { inw(base) };
        sector[i * 2]     = word as u8;
        sector[i * 2 + 1] = (word >> 8) as u8;
    }
    // Read status to acknowledge
    let _ = unsafe { inb(base + 7) };
    Ok(())
}

/// Read `count` consecutive 2048-byte CD sectors starting at `lba` into `output`.
/// `output.len()` must be >= `count * CD_SECTOR_SIZE`.
pub fn read_sectors(lba: u32, count: usize, output: &mut [u8]) -> Result<(), &'static str> {
    let (base, drive_sel) = get_drive().ok_or("ATAPI: no CD-ROM drive found")?;
    for i in 0..count {
        read_one(base, drive_sel, lba + i as u32, &mut output[i * CD_SECTOR_SIZE..(i + 1) * CD_SECTOR_SIZE])?;
    }
    Ok(())
}
