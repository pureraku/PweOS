//! Temporary single-process user-mode execution support.
//!
//! This is the first runtime step for running freestanding C programs. User
//! memory is a dedicated identity-mapped arena; a later ELF loader will replace
//! the demonstration program used here.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::isr::Registers;
use crate::arch::paging::{user_range_is_valid, USER_MEMORY_BASE, USER_MEMORY_END};

const SYS_EXIT: u32 = 0;
const SYS_WRITE: u32 = 1;
const ELF32_HEADER_SIZE: usize = 52;
const ELF32_PROGRAM_HEADER_SIZE: usize = 32;
const MAX_LOAD_SEGMENTS: usize = 16;
const USER_STACK_SIZE: usize = 64 * 1024;
const USER_STACK_TOP: usize = USER_MEMORY_END - 16;
const USER_STACK_BOTTOM: usize = USER_MEMORY_END - USER_STACK_SIZE;
const DEMO_CODE_LEN: usize = 29;
const DEMO_MESSAGE: &[u8] = b"Hello from ring 3!\n";

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

        let status = user_enter(entry as u32, USER_STACK_TOP as u32) as i32;
        PROCESS_ACTIVE.store(false, Ordering::Release);
        Ok(status)
    }
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
            || memory_end > USER_STACK_BOTTOM
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
        _ => regs.eax = u32::MAX,
    }
}
