extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::ata::{read_storage, write_storage};
use crate::sync::Spinlock;

const SECTOR_SIZE: usize = 512;
const MAGIC: &[u8; 8] = b"PWEFS001";
const TOTAL_SECTORS: u32 = 16 * 1024 * 1024 / 512;
const DIRECTORY_START: u32 = 1;
const DIRECTORY_SECTORS: u32 = 8;
const DATA_START: u32 = DIRECTORY_START + DIRECTORY_SECTORS;
const ENTRY_SIZE: usize = 64;
const NAME_SIZE: usize = 32;
const MAX_ENTRIES: usize = DIRECTORY_SECTORS as usize * SECTOR_SIZE / ENTRY_SIZE;
const ENTRY_FILE: u8 = 1;
const ENTRY_DIR: u8 = 2;

#[derive(Clone)]
struct FileEntry { name: String, size: u32, start_sector: u32, sectors: u32 }
#[derive(Clone)]
struct DirEntry { path: String }

pub struct PweFs { mounted: bool, files: Vec<FileEntry>, directories: Vec<DirEntry> }

impl PweFs {
    pub const fn new() -> Self {
        Self { mounted: false, files: Vec::new(), directories: Vec::new() }
    }
    pub fn is_mounted(&self) -> bool { self.mounted }

    pub fn format(&mut self) -> Result<(), &'static str> {
        let mut sb = [0u8; SECTOR_SIZE];
        sb[..8].copy_from_slice(MAGIC);
        sb[8..12].copy_from_slice(&1u32.to_le_bytes());
        sb[12..16].copy_from_slice(&TOTAL_SECTORS.to_le_bytes());
        sb[16..20].copy_from_slice(&DATA_START.to_le_bytes());
        write_storage(0, 1, &sb)?;
        let empty = [0u8; DIRECTORY_SECTORS as usize * SECTOR_SIZE];
        write_storage(DIRECTORY_START, DIRECTORY_SECTORS as usize, &empty)?;
        self.files.clear(); self.directories.clear(); self.mounted = true;
        Ok(())
    }

    pub fn mount(&mut self) -> Result<(), &'static str> {
        let mut sb = [0u8; SECTOR_SIZE];
        read_storage(0, 1, &mut sb)?;
        if &sb[..8] != MAGIC { self.mounted = false; return Err("PweFS not formatted"); }
        let version = read_u32(&sb, 8);
        let total = read_u32(&sb, 12);
        let data_start = read_u32(&sb, 16);
        if version != 1 || total != TOTAL_SECTORS || data_start != DATA_START {
            self.mounted = false; return Err("invalid PweFS superblock");
        }
        let mut table = [0u8; DIRECTORY_SECTORS as usize * SECTOR_SIZE];
        read_storage(DIRECTORY_START, DIRECTORY_SECTORS as usize, &mut table)?;
        self.files.clear(); self.directories.clear();
        for slot in table.chunks_exact(ENTRY_SIZE) {
            match slot[0] {
                0 => continue,
                ENTRY_FILE => {
                    let name = decode_name(&slot[1..33])?;
                    if !valid_path(&name) { return self.mount_error("invalid filename"); }
                    let size = read_u32(slot, 33);
                    let start = read_u32(slot, 37);
                    let sectors = read_u32(slot, 41);
                    if (sectors == 0 && size != 0) || start < DATA_START
                        || start.checked_add(sectors).filter(|&e| e <= TOTAL_SECTORS).is_none()
                        || size > sectors.saturating_mul(SECTOR_SIZE as u32) {
                        return self.mount_error("invalid file allocation");
                    }
                    if self.files.iter().any(|f| f.name == name) || self.directories.iter().any(|d| d.path == name) {
                        return self.mount_error("duplicate path");
                    }
                    self.files.push(FileEntry { name, size, start_sector: start, sectors });
                }
                ENTRY_DIR => {
                    let path = decode_name(&slot[1..33])?;
                    if !valid_path(&path) { return self.mount_error("invalid directory path"); }
                    if self.directories.iter().any(|d| d.path == path) || self.files.iter().any(|f| f.name == path) {
                        return self.mount_error("duplicate path");
                    }
                    self.directories.push(DirEntry { path });
                }
                _ => return self.mount_error("unknown directory entry"),
            }
        }
        for path in self.files.iter().map(|f| f.name.as_str())
            .chain(self.directories.iter().map(|d| d.path.as_str())) {
            let parent = parent_path(path);
            if !parent.is_empty() && !self.directories.iter().any(|d| d.path == parent) {
                return self.mount_error("parent directory missing");
            }
        }
        self.mounted = true; Ok(())
    }

    fn mount_error<T>(&mut self, message: &'static str) -> Result<T, &'static str> {
        self.mounted = false; self.files.clear(); self.directories.clear(); Err(message)
    }

    pub fn list<F: FnMut(&str, usize)>(&self, mut cb: F) {
        self.list_in("", |name, size, dir| { if !dir { cb(name, size); } });
    }
    pub fn list_in<F: FnMut(&str, usize, bool)>(&self, path: &str, mut cb: F) {
        if !self.mounted { return; }
        for d in &self.directories { if parent_path(&d.path) == path { cb(base_name(&d.path), 0, true); } }
        for f in &self.files { if parent_path(&f.name) == path { cb(base_name(&f.name), f.size as usize, false); } }
    }
    pub fn read(&self, name: &str) -> Result<Vec<u8>, &'static str> {
        self.ensure_mounted()?;
        let f = self.files.iter().find(|f| f.name == name).ok_or("file not found")?;
        let mut data = Vec::new();
        data.try_reserve_exact(f.sectors as usize * SECTOR_SIZE).map_err(|_| "not enough memory")?;
        data.resize(f.sectors as usize * SECTOR_SIZE, 0);
        if f.sectors > 0 { read_storage(f.start_sector, f.sectors as usize, &mut data)?; }
        data.truncate(f.size as usize); Ok(data)
    }
    pub fn write(&mut self, name: &str, data: &[u8]) -> Result<(), &'static str> {
        self.ensure_mounted()?;
        if !valid_path(name) || name.len() > NAME_SIZE { return Err("invalid path (max 32 bytes)"); }
        let parent = parent_path(name);
        if !parent.is_empty() && !self.directories.iter().any(|d| d.path == parent) {
            return Err("parent directory does not exist");
        }
        if self.directories.iter().any(|d| d.path == name) { return Err("path is a directory"); }
        if data.len() > u32::MAX as usize { return Err("file too large"); }
        let sectors = data.len().div_ceil(SECTOR_SIZE) as u32;
        let existing = self.files.iter().position(|f| f.name == name);
        if existing.is_none() && self.files.len() + self.directories.len() >= MAX_ENTRIES { return Err("directory table full"); }
        let start = if let Some(i) = existing {
            if sectors <= self.files[i].sectors { self.files[i].start_sector } else { self.find_free_run(sectors, Some(i))? }
        } else { self.find_free_run(sectors, None)? };
        if sectors > 0 {
            let mut padded = Vec::new();
            padded.try_reserve_exact(sectors as usize * SECTOR_SIZE).map_err(|_| "not enough memory")?;
            padded.resize(sectors as usize * SECTOR_SIZE, 0); padded[..data.len()].copy_from_slice(data);
            write_storage(start, sectors as usize, &padded)?;
        }
        let entry = FileEntry { name: String::from(name), size: data.len() as u32, start_sector: start, sectors };
        if let Some(i) = existing { self.files[i] = entry; } else { self.files.push(entry); }
        self.save_directory()
    }
    pub fn create_dir(&mut self, path: &str) -> Result<(), &'static str> {
        self.ensure_mounted()?;
        if !valid_path(path) || path.len() > NAME_SIZE { return Err("invalid directory path (max 32 bytes)"); }
        if self.files.iter().any(|f| f.name == path) || self.directories.iter().any(|d| d.path == path) {
            return Err("file or directory already exists");
        }
        let parent = parent_path(path);
        if !parent.is_empty() && !self.directories.iter().any(|d| d.path == parent) {
            return Err("parent directory does not exist");
        }
        if self.files.len() + self.directories.len() >= MAX_ENTRIES { return Err("directory table full"); }
        self.directories.push(DirEntry { path: String::from(path) });
        if let Err(e) = self.save_directory() { self.directories.pop(); return Err(e); }
        Ok(())
    }
    pub fn is_dir(&self, path: &str) -> bool {
        self.mounted && (path.is_empty() || self.directories.iter().any(|d| d.path == path))
    }
    pub fn count_in(&self, path: &str) -> usize {
        self.directories.iter().filter(|d| parent_path(&d.path) == path).count()
            + self.files.iter().filter(|f| parent_path(&f.name) == path).count()
    }
    pub fn count(&self) -> usize { self.files.len() + self.directories.len() }
    pub fn delete(&mut self, name: &str) -> Result<(), &'static str> {
        self.ensure_mounted()?;
        let i = self.files.iter().position(|f| f.name == name).ok_or("file not found")?;
        self.files.remove(i); self.save_directory()
    }
    pub fn delete_dir(&mut self, path: &str) -> Result<(), &'static str> {
        self.ensure_mounted()?;
        if self.count_in(path) != 0 { return Err("directory is not empty"); }
        let i = self.directories.iter().position(|d| d.path == path).ok_or("directory not found")?;
        self.directories.remove(i); self.save_directory()
    }
    pub fn nuke(&mut self, path: &str) -> Result<(), &'static str> {
        self.ensure_mounted()?;
        if path.is_empty() { return Err("invalid path"); }
        let exists = self.files.iter().any(|f| f.name == path) || self.directories.iter().any(|d| d.path == path);
        if !exists { return Err("path not found"); }
        let mut prefix = String::from(path); prefix.push('/');
        self.files.retain(|f| f.name != path && !f.name.starts_with(&prefix));
        self.directories.retain(|d| d.path != path && !d.path.starts_with(&prefix));
        self.save_directory()
    }
    fn ensure_mounted(&self) -> Result<(), &'static str> {
        if self.mounted { Ok(()) } else { Err("PweFS is not mounted") }
    }
    fn find_free_run(&self, needed: u32, ignore: Option<usize>) -> Result<u32, &'static str> {
        if needed == 0 { return Ok(DATA_START); }
        let mut candidate = DATA_START;
        loop {
            let end = candidate.checked_add(needed).ok_or("disk space exhausted")?;
            if end > TOTAL_SECTORS { return Err("disk space exhausted"); }
            let mut conflict_end = candidate; let mut conflict = false;
            for (i, f) in self.files.iter().enumerate() {
                if Some(i) == ignore || f.sectors == 0 { continue; }
                let fe = f.start_sector.checked_add(f.sectors).ok_or("invalid allocation")?;
                if candidate < fe && end > f.start_sector { conflict = true; conflict_end = conflict_end.max(fe); }
            }
            if !conflict { return Ok(candidate); }
            candidate = conflict_end;
        }
    }
    fn save_directory(&self) -> Result<(), &'static str> {
        let mut table = [0u8; DIRECTORY_SECTORS as usize * SECTOR_SIZE];
        let mut n = 0;
        for f in &self.files {
            if n >= MAX_ENTRIES { return Err("directory table full"); }
            encode_entry(&mut table[n*ENTRY_SIZE..(n+1)*ENTRY_SIZE], ENTRY_FILE, &f.name, f.size, f.start_sector, f.sectors)?;
            n += 1;
        }
        for d in &self.directories {
            if n >= MAX_ENTRIES { return Err("directory table full"); }
            encode_entry(&mut table[n*ENTRY_SIZE..(n+1)*ENTRY_SIZE], ENTRY_DIR, &d.path, 0, DATA_START, 0)?;
            n += 1;
        }
        write_storage(DIRECTORY_START, DIRECTORY_SECTORS as usize, &table)
    }
}
fn encode_entry(slot: &mut [u8], kind: u8, name: &str, size: u32, start: u32, sectors: u32) -> Result<(), &'static str> {
    if name.is_empty() || name.len() > NAME_SIZE { return Err("path too long"); }
    slot.fill(0); slot[0] = kind; slot[1..1+name.len()].copy_from_slice(name.as_bytes());
    slot[33..37].copy_from_slice(&size.to_le_bytes());
    slot[37..41].copy_from_slice(&start.to_le_bytes());
    slot[41..45].copy_from_slice(&sectors.to_le_bytes()); Ok(())
}
fn decode_name(bytes: &[u8]) -> Result<String, &'static str> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    Ok(String::from(core::str::from_utf8(&bytes[..end]).map_err(|_| "invalid path encoding")?))
}
fn read_u32(b: &[u8], i: usize) -> u32 { u32::from_le_bytes([b[i], b[i+1], b[i+2], b[i+3]]) }
fn valid_path(p: &str) -> bool {
    !p.is_empty() && p.split('/').all(|s| !s.is_empty() && s.len() < 32 && s != "." && s != ".."
        && !s.contains('\\') && !s.as_bytes().contains(&0))
}
fn parent_path(p: &str) -> &str { p.rsplit_once('/').map(|(a, _)| a).unwrap_or("") }
fn base_name(p: &str) -> &str { p.rsplit('/').next().unwrap_or(p) }

static PWEFS: Spinlock<PweFs> = Spinlock::new(PweFs::new());
pub fn pwefs_format() -> Result<(), &'static str> { PWEFS.lock().format() }
pub fn pwefs_mount() -> Result<(), &'static str> { PWEFS.lock().mount() }
pub fn pwefs_init() -> Result<(), &'static str> {
    let mut sb = [0u8; SECTOR_SIZE]; read_storage(0, 1, &mut sb)?;
    if sb.iter().all(|&b| b == 0) { PWEFS.lock().format() } else { PWEFS.lock().mount() }
}
pub fn pwefs_is_mounted() -> bool { PWEFS.lock().is_mounted() }
pub fn pwefs_list<F: FnMut(&str, usize)>(cb: F) { PWEFS.lock().list(cb); }
pub fn pwefs_list_in<F: FnMut(&str, usize, bool)>(p: &str, cb: F) { PWEFS.lock().list_in(p, cb); }
pub fn pwefs_read(n: &str) -> Result<Vec<u8>, &'static str> { PWEFS.lock().read(n) }
pub fn pwefs_write(n: &str, d: &[u8]) -> Result<(), &'static str> { PWEFS.lock().write(n, d) }
pub fn pwefs_delete(n: &str) -> Result<(), &'static str> { PWEFS.lock().delete(n) }
pub fn pwefs_create_dir(p: &str) -> Result<(), &'static str> { PWEFS.lock().create_dir(p) }
pub fn pwefs_delete_dir(p: &str) -> Result<(), &'static str> { PWEFS.lock().delete_dir(p) }
pub fn pwefs_nuke(p: &str) -> Result<(), &'static str> { PWEFS.lock().nuke(p) }
pub fn pwefs_is_dir(p: &str) -> bool { PWEFS.lock().is_dir(p) }
pub fn pwefs_count_in(p: &str) -> usize { PWEFS.lock().count_in(p) }
pub fn pwefs_count() -> usize { PWEFS.lock().count() }
pub fn pwefs_exists(n: &str) -> bool { PWEFS.lock().files.iter().any(|f| f.name == n) }
pub fn pwefs_file_len(n: &str) -> Option<usize> { PWEFS.lock().files.iter().find(|f| f.name == n).map(|f| f.size as usize) }
