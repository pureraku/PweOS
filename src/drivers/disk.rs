//! Unified disk reader: tries primary-master ATA PIO first (separate data.img),
//! then falls back to reading data.img embedded in the boot ISO via ATAPI.
//!
//! All public functions use 512-byte logical sector numbers matching data.img.

use crate::sync::Spinlock;
use super::atapi;

/// Cached ISO LBA (2048-byte CD sectors) where data.img starts on the CD.
static DATA_CD_LBA: Spinlock<Option<u32>> = Spinlock::new(None);

/// Scan the CD-ROM for the PWEOSVID magic that marks the start of data.img.
fn find_data_cd_lba() -> Option<u32> {
    {
        let cached = DATA_CD_LBA.lock();
        if let Some(lba) = *cached {
            return Some(lba);
        }
    }
    let mut sector = [0u8; atapi::CD_SECTOR_SIZE];
    // Skip the first 16 ISO system sectors; data.img can't be before that.
    // Scan up to 1024 sectors to safely find data.img (after floppy image ~720 CD sectors).
    for lba in 16u32..1024 {
        if atapi::read_sectors(lba, 1, &mut sector).is_ok() && sector.starts_with(b"PWEOSVID") {
            *DATA_CD_LBA.lock() = Some(lba);
            return Some(lba);
        }
    }
    None
}

/// Read `sector_count` 512-byte sectors starting at `lba` from the data disk
/// (either ATA IDE drive or data.img embedded in the boot ISO).
pub fn read_sectors(lba: u32, sector_count: usize, output: &mut [u8]) -> Result<(), &'static str> {
    // Try ATA PIO first (data.img on a separate IDE drive)
    if crate::drivers::ata::read_sectors(lba, sector_count, output).is_ok() {
        return Ok(());
    }
    // Fall back to ATAPI CD-ROM
    read_from_cd(lba, sector_count, output)
}

fn read_from_cd(lba_512: u32, sector_count: usize, output: &mut [u8]) -> Result<(), &'static str> {
    let base_cd_lba = find_data_cd_lba().ok_or("data disk not found: no IDE drive or ATAPI CD-ROM with PWEOSVID")?;

    // Read sector by sector. Each 512-byte logical sector maps into a 2048-byte CD sector.
    // Optimize: batch consecutive 512-byte sectors that fall within the same 2048-byte CD sector.
    let mut i = 0usize;
    let mut cd_buf = [0u8; atapi::CD_SECTOR_SIZE];
    let mut last_cd_lba: u32 = u32::MAX; // sentinel: no sector cached

    while i < sector_count {
        let logical_sector = lba_512 as usize + i;
        let byte_in_data   = logical_sector * 512;
        let cd_lba         = base_cd_lba + (byte_in_data / atapi::CD_SECTOR_SIZE) as u32;
        let cd_off         = byte_in_data % atapi::CD_SECTOR_SIZE;

        // How many 512-byte sectors can we serve from this single CD sector?
        let remaining_in_cd = atapi::CD_SECTOR_SIZE - cd_off;
        let sects_in_cd     = (remaining_in_cd / 512).min(sector_count - i);

        // Fetch the CD sector if not already cached
        if cd_lba != last_cd_lba {
            atapi::read_sectors(cd_lba, 1, &mut cd_buf)?;
            last_cd_lba = cd_lba;
        }

        // Copy up to `sects_in_cd` × 512 bytes
        let bytes = sects_in_cd * 512;
        output[i * 512..i * 512 + bytes].copy_from_slice(&cd_buf[cd_off..cd_off + bytes]);
        i += sects_in_cd;
    }
    Ok(())
}
