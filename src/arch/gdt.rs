use core::mem::size_of;

const GDT_ENTRIES: usize = 6;
const KERNEL_DATA: u16 = 0x10;
pub const USER_CODE: u16 = 0x1b;
pub const USER_DATA: u16 = 0x23;

#[repr(C, packed)]
struct Tss {
    _previous: u32,
    esp0: u32,
    ss0: u32,
    _rest: [u32; 22],
    _trap: u16,
    _iomap_base: u16,
}

impl Tss {
    const fn new() -> Self {
        Self {
            _previous: 0,
            esp0: 0,
            ss0: KERNEL_DATA as u32,
            _rest: [0; 22],
            _trap: 0,
            _iomap_base: size_of::<Self>() as u16,
        }
    }
}

static mut GDT: [u64; GDT_ENTRIES] = [0; GDT_ENTRIES];
static mut TSS: Tss = Tss::new();

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u32,
}

extern "C" {
    fn gdt_flush(pointer: *const GdtPointer);
}

pub fn gdt_init() {
    unsafe {
        let table = core::ptr::addr_of_mut!(GDT);
        (*table)[0] = 0;
        (*table)[1] = 0x00cf_9a00_0000_ffff; // Ring-0 code
        (*table)[2] = 0x00cf_9200_0000_ffff; // Ring-0 data
        (*table)[3] = 0x00cf_fa00_0000_ffff; // Ring-3 code
        (*table)[4] = 0x00cf_f200_0000_ffff; // Ring-3 data

        let tss = core::ptr::addr_of_mut!(TSS);
        (*tss).esp0 = 0x90000;
        (*tss).ss0 = KERNEL_DATA as u32;
        (*table)[5] = tss_descriptor(tss as u32, size_of::<Tss>() as u32 - 1);

        let pointer = GdtPointer {
            limit: (size_of::<[u64; GDT_ENTRIES]>() - 1) as u16,
            base: table as u32,
        };
        gdt_flush(&pointer);
    }
}

pub fn set_kernel_stack(stack: u32) {
    unsafe {
        core::ptr::addr_of_mut!(TSS.esp0).write_unaligned(stack);
    }
}

#[no_mangle]
pub extern "C" fn gdt_set_kernel_stack(stack: u32) {
    set_kernel_stack(stack);
}

fn tss_descriptor(base: u32, limit: u32) -> u64 {
    let low = (limit & 0xffff) | ((base & 0xffff) << 16);
    let high = ((base >> 16) & 0xff)
        | (0x89 << 8)
        | (limit & 0x000f_0000)
        | (base & 0xff00_0000);
    ((high as u64) << 32) | low as u64
}
