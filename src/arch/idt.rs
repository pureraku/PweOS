#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    pub base_low: u16,
    pub sel: u16,
    pub always0: u8,
    pub flags: u8,
    pub base_high: u16,
}

impl IdtEntry {
    pub const fn empty() -> Self {
        Self {
            base_low: 0,
            sel: 0,
            always0: 0,
            flags: 0,
            base_high: 0,
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtPtr {
    pub limit: u16,
    pub base: u32,
}

pub const IDT_ENTRIES: usize = 256;

static mut IDT_ENTRIES_TABLE: [IdtEntry; IDT_ENTRIES] = [IdtEntry::empty(); IDT_ENTRIES];
static mut IDT_POINTER: IdtPtr = IdtPtr { limit: 0, base: 0 };

extern "C" {
    fn idt_flush(ptr: u32);
}

pub fn idt_set_gate(num: usize, base: u32, sel: u16, flags: u8) {
    if num >= IDT_ENTRIES {
        return;
    }
    unsafe {
        let table = &raw mut IDT_ENTRIES_TABLE;
        (*table)[num] = IdtEntry {
            base_low: (base & 0xFFFF) as u16,
            sel,
            always0: 0,
            flags,
            base_high: ((base >> 16) & 0xFFFF) as u16,
        };
    }
}

pub fn idt_init() {
    unsafe {
        let size = (core::mem::size_of::<IdtEntry>() * IDT_ENTRIES) as u16;
        let ptr = &raw mut IDT_POINTER;
        (*ptr).limit = size - 1;
        (*ptr).base = &raw const IDT_ENTRIES_TABLE as u32;

        core::ptr::write_bytes(&raw mut IDT_ENTRIES_TABLE, 0, 1);

        idt_flush(&raw const IDT_POINTER as u32);
    }
}
