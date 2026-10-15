extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use crate::sync::Spinlock;

pub const RAMFS_MAX_NAME: usize = 32;

pub struct RamfsNode {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct Ramfs {
    files: Vec<RamfsNode>,
    directories: Vec<String>,
}

impl Ramfs {
    pub const fn new() -> Self {
        Self { files: Vec::new(), directories: Vec::new() }
    }
}

impl Default for Ramfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Ramfs {

    pub fn find(&self, name: &str) -> Option<&RamfsNode> {
        self.files.iter().find(|n| n.name == name)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut RamfsNode> {
        self.files.iter_mut().find(|n| n.name == name)
    }

    pub fn create(&mut self, name: &str, initial_data: &[u8]) -> bool {
        if !valid_path(name) || self.directories.iter().any(|dir| dir == name)
            || !self.dir_exists(parent_path(name)) {
            return false;
        }
        if self.find(name).is_some() {
            return false;
        }

        self.files.push(RamfsNode {
            name: String::from(name),
            data: initial_data.to_vec(),
        });
        true
    }

    pub fn create_dir(&mut self, name: &str) -> bool {
        if !valid_path(name) || self.find(name).is_some() || self.directories.iter().any(|dir| dir == name)
            || !self.dir_exists(parent_path(name)) {
            return false;
        }
        self.directories.push(String::from(name));
        true
    }

    pub fn delete_dir(&mut self, path: &str) -> bool {
        if path.is_empty() || self.count_in(path) != 0 {
            return false;
        }
        if let Some(pos) = self.directories.iter().position(|dir| dir == path) {
            self.directories.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn nuke(&mut self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        let file_exists = self.files.iter().any(|file| file.name == path);
        let dir_exists = self.directories.iter().any(|dir| dir == path);
        if !file_exists && !dir_exists {
            return false;
        }

        let mut prefix = String::from(path);
        prefix.push('/');
        self.files.retain(|file| file.name != path && !file.name.starts_with(&prefix));
        self.directories.retain(|dir| dir != path && !dir.starts_with(&prefix));
        true
    }

    pub fn dir_exists(&self, path: &str) -> bool {
        path.is_empty() || self.directories.iter().any(|dir| dir == path)
    }

    pub fn count_in(&self, path: &str) -> usize {
        self.directories.iter().filter(|dir| parent_path(dir) == path).count()
            + self.files.iter().filter(|file| parent_path(&file.name) == path).count()
    }

    pub fn list<F: FnMut(&str, usize, bool)>(&self, path: &str, mut cb: F) {
        for directory in &self.directories {
            if parent_path(directory) == path {
                cb(base_name(directory), 0, true);
            }
        }
        for file in &self.files {
            if parent_path(&file.name) == path {
                cb(base_name(&file.name), file.data.len(), false);
            }
        }
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.find(name).map(|n| n.data.clone())
    }

    pub fn write(&mut self, name: &str, data: &[u8], append: bool) -> bool {
        if name.is_empty() {
            return false;
        }

        if let Some(node) = self.find_mut(name) {
            if append {
                node.data.extend_from_slice(data);
            } else {
                node.data.clear();
                node.data.extend_from_slice(data);
            }
            true
        } else {
            self.create(name, data)
        }
    }

    pub fn delete(&mut self, name: &str) -> bool {
        if let Some(pos) = self.files.iter().position(|n| n.name == name) {
            self.files.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.files.len() + self.directories.len()
    }
}

fn valid_path(path: &str) -> bool {
    !path.is_empty() && path.split('/').all(|part| {
        !part.is_empty() && part.len() < RAMFS_MAX_NAME && part != "." && part != ".."
    })
}

fn parent_path(path: &str) -> &str {
    path.rsplit_once('/').map(|(parent, _)| parent).unwrap_or("")
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

static RAMFS: Spinlock<Ramfs> = Spinlock::new(Ramfs::new());

include!(concat!(env!("OUT_DIR"), "/bundled_images.rs"));

pub fn ramfs_init() {
    let mut fs = RAMFS.lock();
    fs.files.clear();
    fs.directories.clear();

    fs.create(
        "motd",
        b"Welcome to PweOS!\nA lightweight 32-bit x86 educational operating system.\n",
    );
    fs.create(
        "os-release",
        b"NAME=PweOS\nVERSION=0.1.0\nID=pweos\nARCH=i686\n",
    );
    fs.create(
        "readme.txt",
        b"PweOS includes heap memory, RTC, CPUID, RAMFS, and shell commands!\nType 'help' to see all commands.\n",
    );
    fs.create("cat.art", 
            br#"
     /\_/\  
    ( o.o ) 
     > ^ <  
            "#
            );

    fs.create_dir("images");
    for &(name, data) in BUNDLED_IMAGES {
        let mut path = String::from("images/");
        path.push_str(name);
        fs.create(&path, data);
    }
}

pub fn ramfs_create(name: &str, initial_data: &[u8]) -> bool {
    RAMFS.lock().create(name, initial_data)
}

pub fn ramfs_create_dir(name: &str) -> bool {
    RAMFS.lock().create_dir(name)
}

pub fn ramfs_delete_dir(path: &str) -> bool {
    RAMFS.lock().delete_dir(path)
}

pub fn ramfs_nuke(path: &str) -> bool {
    RAMFS.lock().nuke(path)
}

pub fn ramfs_read(name: &str) -> Option<Vec<u8>> {
    RAMFS.lock().read(name)
}

pub fn ramfs_write(name: &str, data: &[u8], append: bool) -> bool {
    RAMFS.lock().write(name, data, append)
}

pub fn ramfs_delete(name: &str) -> bool {
    RAMFS.lock().delete(name)
}

pub fn ramfs_count() -> usize {
    RAMFS.lock().count()
}

pub fn ramfs_is_dir(path: &str) -> bool {
    RAMFS.lock().dir_exists(path)
}

pub fn ramfs_count_in(path: &str) -> usize {
    RAMFS.lock().count_in(path)
}

pub fn ramfs_list<F: FnMut(&str, usize, bool)>(path: &str, cb: F) {
    RAMFS.lock().list(path, cb);
}
