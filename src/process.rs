//! Temporary single-process user-mode execution support.
//!
//! User memory is a dedicated identity-mapped arena. The runtime currently
//! supports one active process and a small set of PweOS-specific syscalls.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::isr::Registers;
use crate::arch::paging::{user_range_is_valid, user_read_c_string, USER_HEAP_BASE, USER_MEMORY_BASE, USER_MEMORY_END};
use crate::sync::Spinlock;

const SYS_EXIT: u32 = 0;
const SYS_WRITE: u32 = 1;
const SYS_OPEN: u32 = 2;
const SYS_READ: u32 = 3;
const SYS_CLOSE: u32 = 4;
const SYS_BRK: u32 = 5;
const SYS_FWRITE: u32 = 6;
const OPEN_WRONLY: u32 = 0x0001;
const OPEN_RDWR: u32 = 0x0002;
const OPEN_CREATE: u32 = 0x0040;
const OPEN_TRUNCATE: u32 = 0x0200;
const OPEN_APPEND: u32 = 0x0400;
const OPEN_SUPPORTED_FLAGS: u32 = 0x0643;
const ELF32_HEADER_SIZE: usize = 52;
const ELF32_PROGRAM_HEADER_SIZE: usize = 32;
const MAX_LOAD_SEGMENTS: usize = 16;
const USER_STACK_SIZE: usize = 64 * 1024;
const USER_STACK_TOP: usize = USER_MEMORY_END - 16;
const USER_STACK_BOTTOM: usize = USER_MEMORY_END - USER_STACK_SIZE;
const DEMO_CODE_LEN: usize = 29;
const DEMO_MESSAGE: &[u8] = b"Hello from ring 3!\n";
const MAX_OPEN_FILES: usize = 8;
const MAX_PATH_LENGTH: usize = 128;

struct OpenFile {
    path: String,
    offset: usize,
    readable: bool,
    writable: bool,
    append: bool,
}

static OPEN_FILES: Spinlock<Vec<Option<OpenFile>>> = Spinlock::new(Vec::new());
static mut USER_PROGRAM_BREAK: usize = USER_HEAP_BASE;

#[derive(Clone, Copy)]
struct LoadSegment {
    offset: usize,
    address: usize,
    file_size: usize,
    memory_size: usize,
    flags: u32,
}

impl LoadSegment {
    const EMPTY: Self = Self {
        offset: 0,
        address: 0,
        file_size: 0,
        memory_size: 0,
        flags: 0,
    };
}

static PROCESS_ACTIVE: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub static mut USER_RETURN_ESP: u32 = 0;
#[no_mangle]
pub static mut USER_EXIT_STATUS: u32 = 0;
#[no_mangle]
pub static mut USER_RETURN_EFLAGS: u32 = 0;

extern "C" {
    fn user_enter(entry: u32, stack: u32) -> u32;
    fn user_leave(status: u32) -> !;
}

pub fn run_ring3_demo() -> Option<i32> {
    if PROCESS_ACTIVE.swap(true, Ordering::AcqRel) {
        return None;
    }

    let message_address = USER_MEMORY_BASE + DEMO_CODE_LEN;
    let mut code = [0u8; DEMO_CODE_LEN];
    code[0] = 0xb8; // mov eax, SYS_WRITE
    code[1..5].copy_from_slice(&SYS_WRITE.to_le_bytes());
    code[5] = 0xbb; // mov ebx, message pointer
    code[6..10].copy_from_slice(&(message_address as u32).to_le_bytes());
    code[10] = 0xb9; // mov ecx, message length
    code[11..15].copy_from_slice(&(DEMO_MESSAGE.len() as u32).to_le_bytes());
    code[15..17].copy_from_slice(&[0xcd, 0x80]); // int 0x80
    code[17] = 0xb8; // mov eax, SYS_EXIT
    code[18..22].copy_from_slice(&SYS_EXIT.to_le_bytes());
    code[22] = 0xbb; // mov ebx, exit status 0
    code[23..27].copy_from_slice(&0u32.to_le_bytes());
    code[27..29].copy_from_slice(&[0xcd, 0x80]); // int 0x80

    unsafe {
        core::ptr::copy_nonoverlapping(
            code.as_ptr(),
            USER_MEMORY_BASE as *mut u8,
            code.len(),
        );
        core::ptr::copy_nonoverlapping(
            DEMO_MESSAGE.as_ptr(),
            message_address as *mut u8,
            DEMO_MESSAGE.len(),
        );

        let status = user_enter(USER_MEMORY_BASE as u32, (USER_MEMORY_END - 16) as u32) as i32;
        PROCESS_ACTIVE.store(false, Ordering::Release);
        Some(status)
    }
}

pub fn run_elf(image: &[u8]) -> Result<i32, &'static str> {
    let (entry, segments, segment_count) = parse_elf32(image)?;
    if PROCESS_ACTIVE.swap(true, Ordering::AcqRel) {
        return Err("a user program is already active");
    }

    unsafe {
        reset_process_resources();
        core::ptr::write_bytes(
            USER_MEMORY_BASE as *mut u8,
            0,
            USER_STACK_BOTTOM - USER_MEMORY_BASE,
        );
        for segment in &segments[..segment_count] {
            let source = image.as_ptr().add(segment.offset);
            let destination = segment.address as *mut u8;
            core::ptr::copy_nonoverlapping(source, destination, segment.file_size);
            core::ptr::write_bytes(
                destination.add(segment.file_size),
                0,
                segment.memory_size - segment.file_size,
            );
        }

        USER_PROGRAM_BREAK = USER_HEAP_BASE;
        let status = user_enter(entry as u32, USER_STACK_TOP as u32) as i32;
        reset_process_resources();
        PROCESS_ACTIVE.store(false, Ordering::Release);
        Ok(status)
    }
}

fn reset_process_resources() {
    let mut files = OPEN_FILES.lock();
    files.clear();
    files.resize_with(MAX_OPEN_FILES, || None);
}

fn parse_elf32(image: &[u8]) -> Result<(usize, [LoadSegment; MAX_LOAD_SEGMENTS], usize), &'static str> {
    if image.len() < ELF32_HEADER_SIZE || &image[..4] != b"\x7fELF" {
        return Err("not an ELF file");
    }
    if image[4] != 1 || image[5] != 1 || image[6] != 1 {
        return Err("expected a little-endian ELF32 executable");
    }
    if read_u16(image, 16)? != 2 || read_u16(image, 18)? != 3 || read_u32(image, 20)? != 1 {
        return Err("expected a static i386 executable");
    }

    let entry = read_u32(image, 24)? as usize;
    let program_headers = read_u32(image, 28)? as usize;
    let header_size = read_u16(image, 40)? as usize;
    let program_header_size = read_u16(image, 42)? as usize;
    let program_header_count = read_u16(image, 44)? as usize;
    if header_size < ELF32_HEADER_SIZE || program_header_size != ELF32_PROGRAM_HEADER_SIZE {
        return Err("invalid ELF header sizes");
    }
    if program_header_count == 0 || program_header_count > MAX_LOAD_SEGMENTS {
        return Err("unsupported ELF program-header count");
    }
    let table_size = program_header_count
        .checked_mul(program_header_size)
        .ok_or("ELF program-header table overflows")?;
    let table_end = program_headers
        .checked_add(table_size)
        .ok_or("ELF program-header table overflows")?;
    if program_headers < header_size || table_end > image.len() {
        return Err("ELF program-header table is outside the file");
    }

    let mut segments = [LoadSegment::EMPTY; MAX_LOAD_SEGMENTS];
    let mut segment_count = 0;
    let mut entry_is_executable = false;
    for index in 0..program_header_count {
        let header = program_headers + index * program_header_size;
        match read_u32(image, header)? {
            1 => {}
            2 | 3 => return Err("dynamic ELF executables are not supported"),
            _ => continue,
        }
        if segment_count == MAX_LOAD_SEGMENTS {
            return Err("too many ELF load segments");
        }
        let segment = LoadSegment {
            offset: read_u32(image, header + 4)? as usize,
            address: read_u32(image, header + 8)? as usize,
            file_size: read_u32(image, header + 16)? as usize,
            memory_size: read_u32(image, header + 20)? as usize,
            flags: read_u32(image, header + 24)?,
        };
        let file_end = segment
            .offset
            .checked_add(segment.file_size)
            .ok_or("ELF segment file range overflows")?;
        let memory_end = segment
            .address
            .checked_add(segment.memory_size)
            .ok_or("ELF segment memory range overflows")?;
        if segment.file_size > segment.memory_size || file_end > image.len() {
            return Err("invalid ELF segment size");
        }
        if segment.memory_size == 0
            || segment.address < USER_MEMORY_BASE
            || memory_end > USER_HEAP_BASE
        {
            return Err("ELF segment is outside the user program area");
        }
        for previous in &segments[..segment_count] {
            let previous_end = previous.address + previous.memory_size;
            if segment.address < previous_end && previous.address < memory_end {
                return Err("overlapping ELF load segments are not supported");
            }
        }
        if segment.flags & 1 != 0 && entry >= segment.address && entry < memory_end {
            entry_is_executable = true;
        }
        segments[segment_count] = segment;
        segment_count += 1;
    }

    if segment_count == 0 || !entry_is_executable {
        return Err("ELF entry point is not in an executable load segment");
    }
    Ok((entry, segments, segment_count))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, &'static str> {
    let end = offset.checked_add(2).ok_or("ELF field offset overflows")?;
    let value = bytes.get(offset..end).ok_or("truncated ELF header")?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, &'static str> {
    let end = offset.checked_add(4).ok_or("ELF field offset overflows")?;
    let value = bytes.get(offset..end).ok_or("truncated ELF header")?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

pub fn abort_current(status: i32) -> ! {
    unsafe { user_leave(status as u32) }
}

#[no_mangle]
pub unsafe extern "C" fn syscall_handler(regs: *mut Registers) {
    let regs = &mut *regs;
    match regs.eax {
        SYS_EXIT => abort_current(regs.ebx as i32),
        SYS_WRITE => {
            let address = regs.ebx;
            let length = regs.ecx as usize;
            if length <= 4096 && user_range_is_valid(address, length) {
                let bytes = core::slice::from_raw_parts(address as *const u8, length);
                if let Ok(text) = core::str::from_utf8(bytes) {
                    print!("{}", text);
                    regs.eax = length as u32;
                } else {
                    regs.eax = u32::MAX;
                }
            } else {
                regs.eax = u32::MAX;
            }
        }
        SYS_OPEN => {
            let mut path_bytes = [0u8; MAX_PATH_LENGTH];
            let Some(path_length) = user_read_c_string(regs.ebx, &mut path_bytes) else {
                regs.eax = u32::MAX;
                return;
            };
            let flags = regs.ecx;
            let access_mode = flags & 3;
            let writable = access_mode == OPEN_WRONLY || access_mode == OPEN_RDWR;
            let readable = access_mode == 0 || access_mode == OPEN_RDWR;
            if flags & !OPEN_SUPPORTED_FLAGS != 0
                || access_mode == 3
                || (flags & OPEN_TRUNCATE != 0 && !writable)
                || path_length == 0
            {
                regs.eax = u32::MAX;
                return;
            }
            let Ok(path) = core::str::from_utf8(&path_bytes[..path_length]) else {
                regs.eax = u32::MAX;
                return;
            };
            if crate::fs::ramfs::ramfs_file_len(path).is_none() {
                if flags & OPEN_CREATE == 0 || !crate::fs::ramfs::ramfs_create(path, &[]) {
                    regs.eax = u32::MAX;
                    return;
                }
            }
            if flags & OPEN_TRUNCATE != 0
                && !crate::fs::ramfs::ramfs_write_at(path, 0, &[], true)
            {
                regs.eax = u32::MAX;
                return;
            }
            let offset = if flags & OPEN_APPEND != 0 {
                crate::fs::ramfs::ramfs_file_len(path).unwrap_or(0)
            } else {
                0
            };
            let mut files = OPEN_FILES.lock();
            if let Some((fd, slot)) = files.iter_mut().enumerate().skip(3).find(|(_, slot)| slot.is_none()) {
                *slot = Some(OpenFile {
                    path: String::from(path),
                    offset,
                    readable,
                    writable,
                    append: flags & OPEN_APPEND != 0,
                });
                regs.eax = fd as u32;
            } else {
                regs.eax = u32::MAX;
            }
        }
        SYS_READ => {
            let fd = regs.ebx as usize;
            let address = regs.ecx;
            let length = regs.edx as usize;
            if length > 4096 || !user_range_is_valid(address, length) {
                regs.eax = u32::MAX;
                return;
            }
            let mut files = OPEN_FILES.lock();
            let Some(Some(file)) = files.get_mut(fd) else {
                regs.eax = u32::MAX;
                return;
            };
            if !file.readable {
                regs.eax = u32::MAX;
                return;
            }
            let buffer = core::slice::from_raw_parts_mut(address as *mut u8, length);
            match crate::fs::ramfs::ramfs_read_at(&file.path, file.offset, buffer) {
                Some(count) => {
                    file.offset += count;
                    regs.eax = count as u32;
                }
                None => regs.eax = u32::MAX,
            }
        }
        SYS_CLOSE => {
            let fd = regs.ebx as usize;
            let mut files = OPEN_FILES.lock();
            if fd >= 3 && fd < files.len() && files[fd].take().is_some() {
                regs.eax = 0;
            } else {
                regs.eax = u32::MAX;
            }
        }
        SYS_FWRITE => {
            let fd = regs.ebx as usize;
            let address = regs.ecx;
            let length = regs.edx as usize;
            if length > 4096 || !user_range_is_valid(address, length) {
                regs.eax = u32::MAX;
                return;
            }
            let bytes = core::slice::from_raw_parts(address as *const u8, length);
            if fd == 1 || fd == 2 {
                if let Ok(text) = core::str::from_utf8(bytes) {
                    print!("{}", text);
                    regs.eax = length as u32;
                } else {
                    regs.eax = u32::MAX;
                }
                return;
            }
            let mut files = OPEN_FILES.lock();
            let Some(Some(file)) = files.get_mut(fd) else {
                regs.eax = u32::MAX;
                return;
            };
            if !file.writable {
                regs.eax = u32::MAX;
                return;
            }
            if file.append {
                file.offset = crate::fs::ramfs::ramfs_file_len(&file.path).unwrap_or(file.offset);
            }
            if crate::fs::ramfs::ramfs_write_at(&file.path, file.offset, bytes, false) {
                file.offset += length;
                regs.eax = length as u32;
            } else {
                regs.eax = u32::MAX;
            }
        }
        SYS_BRK => {
            let requested = regs.ebx as usize;
            unsafe {
                if requested == 0 {
                    regs.eax = USER_PROGRAM_BREAK as u32;
                } else if requested >= USER_HEAP_BASE && requested <= USER_STACK_BOTTOM {
                    USER_PROGRAM_BREAK = requested;
                    regs.eax = USER_PROGRAM_BREAK as u32;
                } else {
                    regs.eax = u32::MAX;
                }
            }
        }
        _ => regs.eax = u32::MAX,
    }
}
