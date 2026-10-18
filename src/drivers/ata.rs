//! Polled ATA PIO sector reads and writes.
//!
//! Primary master: companion data disk.
//! Primary slave: persistent user storage.

use crate::arch::io::{inb, inw, io_wait, outb, outw};

const DATA: u16 = 0x1F0;
const ERROR_REGISTER: u16 = 0x1F1;
const SECTOR_COUNT: u16 = 0x1F2;
const LBA_LOW: u16 = 0x1F3;
const LBA_MID: u16 = 0x1F4;
const LBA_HIGH: u16 = 0x1F5;
const DRIVE: u16 = 0x1F6;
const STATUS: u16 = 0x1F7;

const READ_SECTORS: u8 = 0x20;
const WRITE_SECTORS: u8 = 0x30;
const FLUSH_CACHE: u8 = 0xE7;

const BUSY: u8 = 0x80;
const DRQ: u8 = 0x08;
const ERROR: u8 = 0x01;
const DEVICE_FAULT: u8 = 0x20;
const LBA28_LIMIT: u32 = 0x1000_0000;

const PRIMARY_MASTER: u8 = 0xE0;
const PRIMARY_SLAVE: u8 = 0xF0;

/// Select an IDE device on the primary ATA channel.
#[derive(Clone, Copy)]
pub enum Device {
    Master,
    Slave,
}

impl Device {
    fn drive_byte(self, lba: u32) -> u8 {
        let device = match self {
            Device::Master => PRIMARY_MASTER,
            Device::Slave => PRIMARY_SLAVE,
        };

        device | ((lba >> 24) as u8 & 0x0F)
    }
}

/// Read complete 512-byte sectors from the primary-master data disk.
pub fn read_sectors(
    lba: u32,
    sectors: usize,
    output: &mut [u8],
) -> Result<(), &'static str> {
    read_device(Device::Master, lba, sectors, output)
}

/// Read sectors from a selected primary-channel IDE device.
pub fn read_device(
    device: Device,
    mut lba: u32,
    mut sectors: usize,
    output: &mut [u8],
) -> Result<(), &'static str> {
    validate_range(lba, sectors, output.len())?;

    let mut output_offset = 0usize;

    while sectors > 0 {
        let count = sectors.min(255);

        select_device(device, lba)?;
        wait_not_busy()?;

        unsafe {
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

/// Write complete 512-byte sectors to a selected primary-channel IDE device.
pub fn write_device(
    device: Device,
    mut lba: u32,
    mut sectors: usize,
    input: &[u8],
) -> Result<(), &'static str> {
    validate_range(lba, sectors, input.len())?;

    let mut input_offset = 0usize;

    while sectors > 0 {
        let count = sectors.min(255);

        select_device(device, lba)?;
        wait_not_busy()?;

        unsafe {
            outb(SECTOR_COUNT, count as u8);
            outb(LBA_LOW, lba as u8);
            outb(LBA_MID, (lba >> 8) as u8);
            outb(LBA_HIGH, (lba >> 16) as u8);
            outb(STATUS, WRITE_SECTORS);
        }

        for _ in 0..count {
            wait_data_request()?;

            for word_index in 0..256 {
                let at = input_offset + word_index * 2;
                let word = u16::from_le_bytes([input[at], input[at + 1]]);

                unsafe {
                    outw(DATA, word);
                }
            }

            input_offset += 512;
        }

        wait_not_busy()?;

        lba += count as u32;
        sectors -= count;
    }

    // Flush the drive's write cache before reporting success.
    select_device(device, lba.saturating_sub(1))?;
    wait_not_busy()?;

    unsafe {
        outb(STATUS, FLUSH_CACHE);
    }

    wait_not_busy()?;

    Ok(())
}

/// Convenience wrapper for writing to the primary-master data disk.
pub fn write_sectors(
    lba: u32,
    sectors: usize,
    input: &[u8],
) -> Result<(), &'static str> {
    write_device(Device::Master, lba, sectors, input)
}

/// Read sectors from the persistent user-storage disk.
pub fn read_storage(
    lba: u32,
    sectors: usize,
    output: &mut [u8],
) -> Result<(), &'static str> {
    read_device(Device::Slave, lba, sectors, output)
}

/// Write sectors to the persistent user-storage disk.
pub fn write_storage(
    lba: u32,
    sectors: usize,
    input: &[u8],
) -> Result<(), &'static str> {
    write_device(Device::Slave, lba, sectors, input)
}

fn validate_range(
    lba: u32,
    sectors: usize,
    buffer_len: usize,
) -> Result<(), &'static str> {
    if sectors == 0
        || sectors > buffer_len / 512
        || lba >= LBA28_LIMIT
        || (sectors as u64) > (LBA28_LIMIT - lba) as u64
    {
        return Err("invalid ATA sector range");
    }

    Ok(())
}

fn select_device(device: Device, lba: u32) -> Result<(), &'static str> {
    unsafe {
        outb(DRIVE, device.drive_byte(lba));
        io_wait();
        io_wait();
        io_wait();
        io_wait();
    }

    wait_not_busy()
}

fn wait_not_busy() -> Result<(), &'static str> {
    for _ in 0..10_000_000 {
        let status = unsafe { inb(STATUS) };

        if status == 0 || status == 0xFF {
            return Err("ATA device is not attached");
        }

        if status & BUSY == 0 {
            if status & (ERROR | DEVICE_FAULT) != 0 {
                let _error = unsafe { inb(ERROR_REGISTER) };
                return Err("ATA device reported an error");
            }

            return Ok(());
        }

        core::hint::spin_loop();
    }

    Err("timed out waiting for ATA device")
}

fn wait_data_ready() -> Result<(), &'static str> {
    for _ in 0..10_000_000 {
        let status = unsafe { inb(STATUS) };

        if status == 0 || status == 0xFF {
            return Err("ATA device is not attached");
        }

        if status & BUSY == 0 {
            if status & (ERROR | DEVICE_FAULT) != 0 {
                let _error = unsafe { inb(ERROR_REGISTER) };
                return Err("ATA read failed");
            }

            if status & DRQ != 0 {
                return Ok(());
            }
        }

        core::hint::spin_loop();
    }

    Err("timed out reading ATA sector")
}

fn wait_data_request() -> Result<(), &'static str> {
    for _ in 0..10_000_000 {
        let status = unsafe { inb(STATUS) };

        if status == 0 || status == 0xFF {
            return Err("ATA device is not attached");
        }

        if status & BUSY == 0 {
            if status & (ERROR | DEVICE_FAULT) != 0 {
                let _error = unsafe { inb(ERROR_REGISTER) };
                return Err("ATA write failed");
            }

            if status & DRQ != 0 {
                return Ok(());
            }
        }

        core::hint::spin_loop();
    }

    Err("timed out waiting to write ATA sector")
}
