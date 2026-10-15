//! Polled primary-master ATA PIO reads for the companion video data disk.

use crate::arch::io::{inb, inw, io_wait, outb};

const DATA: u16 = 0x1F0;
const SECTOR_COUNT: u16 = 0x1F2;
const LBA_LOW: u16 = 0x1F3;
const LBA_MID: u16 = 0x1F4;
const LBA_HIGH: u16 = 0x1F5;
const DRIVE: u16 = 0x1F6;
const STATUS: u16 = 0x1F7;
const READ_SECTORS: u8 = 0x20;
const BUSY: u8 = 0x80;
const READY: u8 = 0x08;
const ERROR: u8 = 0x01;
const DEVICE_FAULT: u8 = 0x20;

/// Read complete 512-byte sectors from the primary master into `output`.
pub fn read_sectors(mut lba: u32, mut sectors: usize, output: &mut [u8]) -> Result<(), &'static str> {
    if sectors == 0 || output.len() < sectors * 512 || lba >= 0x1000_0000 {
        return Err("invalid ATA read range");
    }
    let mut output_offset = 0usize;
    while sectors > 0 {
        let count = sectors.min(255);
        wait_not_busy()?;
        unsafe {
            outb(DRIVE, 0xE0 | ((lba >> 24) as u8 & 0x0F));
            io_wait(); io_wait(); io_wait(); io_wait();
            outb(SECTOR_COUNT, count as u8);
            outb(LBA_LOW, lba as u8);
            outb(LBA_MID, (lba >> 8) as u8);
            outb(LBA_HIGH, (lba >> 16) as u8);
            outb(STATUS, READ_SECTORS);
        }

        for _ in 0..count {
            wait_data_ready()?;
            for word_index in 0..256 {
                let word = unsafe { inw(DATA) };
                let at = output_offset + word_index * 2;
                output[at] = word as u8;
                output[at + 1] = (word >> 8) as u8;
            }
            output_offset += 512;
        }
        lba += count as u32;
        sectors -= count;
    }
    Ok(())
}

fn wait_not_busy() -> Result<(), &'static str> {
    for _ in 0..10_000_000 {
        let status = unsafe { inb(STATUS) };
        if status == 0 || status == 0xFF {
            return Err("video data disk is not attached");
        }
        if status & BUSY == 0 {
            if status & (ERROR | DEVICE_FAULT) != 0 {
                return Err("video data disk reported an ATA error");
            }
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("timed out waiting for video data disk")
}

fn wait_data_ready() -> Result<(), &'static str> {
    for _ in 0..10_000_000 {
        let status = unsafe { inb(STATUS) };
        if status == 0 || status == 0xFF {
            return Err("video data disk is not attached");
        }
        if status & BUSY == 0 {
            if status & (ERROR | DEVICE_FAULT) != 0 {
                return Err("video data disk reported an ATA error");
            }
            if status & READY != 0 {
                return Ok(());
            }
        }
        core::hint::spin_loop();
    }
    Err("timed out reading video data disk")
}
