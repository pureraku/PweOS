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
}

impl Ramfs {
    pub const fn new() -> Self {
        Self { files: Vec::new() }
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
        if name.is_empty() || name.len() >= RAMFS_MAX_NAME {
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
        self.files.len()
    }
}

static RAMFS: Spinlock<Ramfs> = Spinlock::new(Ramfs::new());

pub fn ramfs_init() {
    let mut fs = RAMFS.lock();
    fs.files.clear();

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
}

pub fn ramfs_create(name: &str, initial_data: &[u8]) -> bool {
    RAMFS.lock().create(name, initial_data)
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

pub fn ramfs_list<F: FnMut(&str, usize)>(mut cb: F) {
    let fs = RAMFS.lock();
    for node in &fs.files {
        cb(&node.name, node.data.len());
    }
}
