extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::ata::{read_storage, write_storage};
use crate::sync::Spinlock;

const SECTOR_SIZE: usize = 512;
const MAGIC: &[u8; 8] = b"PWEFS001";

const TOTAL_SECTORS: u32 = 16 * 1024 * 1024 / 512;
const SUPERBLOCK_SECTOR: u32 = 0;
const DIRECTORY_START: u32 = 1;
const DIRECTORY_SECTORS: u32 = 8;
const DATA_START: u32 = DIRECTORY_START + DIRECTORY_SECTORS;

const ENTRY_SIZE: usize = 64;
const NAME_SIZE: usize = 32;
const MAX_FILES: usize = DIRECTORY_SECTORS as usize * SECTOR_SIZE / ENTRY_SIZE;

const ENTRY_USED: u8 = 1;

#[derive(Clone)]
struct FileEntry {
    name: String,
    size: u32,
    start_sector: u32,
    sectors: u32,
}

pub struct PweFs {
    mounted: bool,
    files: Vec<FileEntry>,
}

impl PweFs {
    pub const fn new() -> Self {
        Self {
            mounted: false,
            files: Vec::new(),
        }
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted
    }

    /// Explicitly initialize the filesystem.
    /// WARNING: this destroys the existing filesystem metadata.
    pub fn format(&mut self) -> Result<(), &'static str> {
        let mut superblock = [0u8; SECTOR_SIZE];

        superblock[..8].copy_from_slice(MAGIC);
        superblock[8..12].copy_from_slice(&1u32.to_le_bytes());
        superblock[12..16].copy_from_slice(&TOTAL_SECTORS.to_le_bytes());
        superblock[16..20].copy_from_slice(&DATA_START.to_le_bytes());

        write_storage(SUPERBLOCK_SECTOR, 1, &superblock)?;

        let empty_directory = [0u8; DIRECTORY_SECTORS as usize * SECTOR_SIZE];

        write_storage(
            DIRECTORY_START,
            DIRECTORY_SECTORS as usize,
            &empty_directory,
        )?;

        self.files.clear();
        self.mounted = true;

        Ok(())
    }

    /// Mount and validate an existing filesystem.
    pub fn mount(&mut self) -> Result<(), &'static str> {
        let mut superblock = [0u8; SECTOR_SIZE];
        read_storage(SUPERBLOCK_SECTOR, 1, &mut superblock)?;

        if &superblock[..8] != MAGIC {
            self.mounted = false;
            return Err("PweFS not formatted");
        }

        let version = u32::from_le_bytes([
            superblock[8],
            superblock[9],
            superblock[10],
            superblock[11],
        ]);

        let total_sectors = u32::from_le_bytes([
            superblock[12],
            superblock[13],
            superblock[14],
            superblock[15],
        ]);

        let data_start = u32::from_le_bytes([
            superblock[16],
            superblock[17],
            superblock[18],
            superblock[19],
        ]);

        if version != 1
            || total_sectors != TOTAL_SECTORS
            || data_start != DATA_START
        {
            self.mounted = false;
            return Err("invalid PweFS superblock");
        }

        let mut directory =
            [0u8; DIRECTORY_SECTORS as usize * SECTOR_SIZE];

        read_storage(
            DIRECTORY_START,
            DIRECTORY_SECTORS as usize,
            &mut directory,
        )?;

        self.files.clear();

        for slot in directory.chunks_exact(ENTRY_SIZE) {
            if slot[0] != ENTRY_USED {
                continue;
            }

            let name_end = slot[1..1 + NAME_SIZE]
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(NAME_SIZE);

            let name = core::str::from_utf8(&slot[1..1 + name_end])
                .map_err(|_| "invalid filename encoding")?;

            if !valid_name(name) {
                self.mounted = false;
                self.files.clear();
                return Err("invalid filename in directory");
            }

            let size = u32::from_le_bytes([
                slot[33], slot[34], slot[35], slot[36],
            ]);

            let start_sector = u32::from_le_bytes([
                slot[37], slot[38], slot[39], slot[40],
            ]);

            let sectors = u32::from_le_bytes([
                slot[41], slot[42], slot[43], slot[44],
            ]);

            if sectors == 0 && size != 0 {
                self.mounted = false;
                self.files.clear();
                return Err("invalid file allocation");
            }

            if start_sector < DATA_START
                || start_sector
                    .checked_add(sectors)
                    .filter(|&end| end <= TOTAL_SECTORS)
                    .is_none()
                || size > sectors.saturating_mul(SECTOR_SIZE as u32)
            {
                self.mounted = false;
                self.files.clear();
                return Err("file allocation is outside the disk");
            }

            if self.files.iter().any(|file| file.name == name) {
                self.mounted = false;
                self.files.clear();
                return Err("duplicate filename in directory");
            }

            self.files.push(FileEntry {
                name: String::from(name),
                size,
                start_sector,
                sectors,
            });
        }

        self.mounted = true;
        Ok(())
    }

    pub fn list<F: FnMut(&str, usize)>(&self, mut callback: F) {
        if !self.mounted {
            return;
        }

        for file in &self.files {
            callback(file.name.as_str(), file.size as usize);
        }
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>, &'static str> {
        self.ensure_mounted()?;

        let file = self.files.iter()
            .find(|file| file.name == name)
            .ok_or("file not found")?;

        let mut data = Vec::new();
        data.try_reserve_exact(file.size as usize)
            .map_err(|_| "not enough memory")?;

        if file.size == 0 {
            return Ok(data);
        }

        let padded_size = file.sectors as usize * SECTOR_SIZE;

        data.resize(padded_size, 0);

        read_storage(file.start_sector, file.sectors as usize, &mut data)?;

        data.truncate(file.size as usize);

        Ok(data)
    }

    pub fn write(&mut self, name: &str, data: &[u8]) -> Result<(), &'static str> {
        self.ensure_mounted()?;

        if !valid_name(name) {
            return Err("invalid filename");
        }

        if data.len() > u32::MAX as usize {
            return Err("file is too large");
        }

        let sectors = data.len().div_ceil(SECTOR_SIZE);
        let sectors_u32 = sectors as u32;

        let existing = self.files.iter().position(|file| file.name == name);

        if existing.is_none() && self.files.len() >= MAX_FILES {
            return Err("directory is full");
        }

        let start_sector = if let Some(index) = existing {
            let file = &self.files[index];

            if sectors_u32 <= file.sectors {
                file.start_sector
            } else {
                self.find_free_run(sectors_u32, Some(index))?
            }
        } else {
            self.find_free_run(sectors_u32, None)?
        };

        if sectors > 0 {
            let mut padded = Vec::new();

            padded.try_reserve_exact(sectors * SECTOR_SIZE)
                .map_err(|_| "not enough memory")?;

            padded.resize(sectors * SECTOR_SIZE, 0);
            padded[..data.len()].copy_from_slice(data);

            write_storage(start_sector, sectors, &padded)?;
        }

        let new_entry = FileEntry {
            name: String::from(name),
            size: data.len() as u32,
            start_sector,
            sectors: sectors_u32,
        };

        if let Some(index) = existing {
            self.files[index] = new_entry;
        } else {
            self.files.push(new_entry);
        }

        self.save_directory()
    }

    pub fn delete(&mut self, name: &str) -> Result<(), &'static str> {
        self.ensure_mounted()?;

        let index = self.files.iter()
            .position(|file| file.name == name)
            .ok_or("file not found")?;

        self.files.remove(index);

        self.save_directory()
    }

    fn ensure_mounted(&self) -> Result<(), &'static str> {
        if self.mounted {
            Ok(())
        } else {
            Err("PweFS is not mounted")
        }
    }

    fn find_free_run(
        &self,
        needed: u32,
        ignore_index: Option<usize>,
    ) -> Result<u32, &'static str> {
        if needed == 0 {
            return Ok(DATA_START);
        }

        let mut candidate = DATA_START;

        loop {
            let end = candidate
                .checked_add(needed)
                .ok_or("disk space exhausted")?;

            if end > TOTAL_SECTORS {
                return Err("disk space exhausted");
            }

            let mut conflict_end = candidate;
            let mut conflict = false;

            for (index, file) in self.files.iter().enumerate() {
                if Some(index) == ignore_index || file.sectors == 0 {
                    continue;
                }

                let file_end = file.start_sector
                    .checked_add(file.sectors)
                    .ok_or("invalid file allocation")?;

                if candidate < file_end && end > file.start_sector {
                    conflict = true;
                    conflict_end = conflict_end.max(file_end);
                }
            }

            if !conflict {
                return Ok(candidate);
            }

            candidate = conflict_end;
        }
    }

    fn save_directory(&self) -> Result<(), &'static str> {
        let mut directory =
            [0u8; DIRECTORY_SECTORS as usize * SECTOR_SIZE];

        for (index, file) in self.files.iter().enumerate() {
            if index >= MAX_FILES {
                return Err("directory is full");
            }

            let start = index * ENTRY_SIZE;
            let slot = &mut directory[start..start + ENTRY_SIZE];

            slot[0] = ENTRY_USED;

            let name_bytes = file.name.as_bytes();
            if name_bytes.len() > NAME_SIZE {
                return Err("filename too long");
            }

            slot[1..1 + name_bytes.len()].copy_from_slice(name_bytes);

            slot[33..37].copy_from_slice(&file.size.to_le_bytes());
            slot[37..41].copy_from_slice(&file.start_sector.to_le_bytes());
            slot[41..45].copy_from_slice(&file.sectors.to_le_bytes());
        }

        write_storage(
            DIRECTORY_START,
            DIRECTORY_SECTORS as usize,
            &directory,
        )
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_SIZE
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.as_bytes().contains(&0)
}

static PWEFS: Spinlock<PweFs> = Spinlock::new(PweFs::new());

pub fn pwefs_format() -> Result<(), &'static str> {
    PWEFS.lock().format()
}

pub fn pwefs_mount() -> Result<(), &'static str> {
    PWEFS.lock().mount()
}

pub fn pwefs_is_mounted() -> bool {
    PWEFS.lock().is_mounted()
}

pub fn pwefs_list<F: FnMut(&str, usize)>(callback: F) {
    PWEFS.lock().list(callback);
}

pub fn pwefs_read(name: &str) -> Result<Vec<u8>, &'static str> {
    PWEFS.lock().read(name)
}

pub fn pwefs_write(name: &str, data: &[u8]) -> Result<(), &'static str> {
    PWEFS.lock().write(name, data)
}

pub fn pwefs_delete(name: &str) -> Result<(), &'static str> {
    PWEFS.lock().delete(name)
}
