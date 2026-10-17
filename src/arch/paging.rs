//! Identity-mapped bootstrap paging for the kernel and a single user process.

const PAGE_SIZE: usize = 4096;
const PAGE_ENTRIES: usize = 1024;
const IDENTITY_SIZE: usize = 32 * 1024 * 1024;
pub const USER_MEMORY_BASE: usize = 4 * 1024 * 1024;
pub const USER_MEMORY_END: usize = IDENTITY_SIZE;
const PRESENT_WRITABLE: u32 = 0x003;
const USER_PAGE: u32 = 0x004;

#[repr(C, align(4096))]
struct Page([u32; PAGE_ENTRIES]);

static mut PAGE_DIRECTORY: Page = Page([0; PAGE_ENTRIES]);
static mut PAGE_TABLES: [Page; IDENTITY_SIZE / (PAGE_SIZE * PAGE_ENTRIES)] =
    [const { Page([0; PAGE_ENTRIES]) }; IDENTITY_SIZE / (PAGE_SIZE * PAGE_ENTRIES)];

pub fn paging_init() {
    unsafe {
        let directory = core::ptr::addr_of_mut!(PAGE_DIRECTORY);
        let tables = core::ptr::addr_of_mut!(PAGE_TABLES).cast::<Page>();
        core::ptr::write_bytes(directory.cast::<u8>(), 0, size_of_page());
        core::ptr::write_bytes(tables.cast::<u8>(), 0, size_of_page() * PAGE_TABLES_COUNT);

        for table_index in 0..PAGE_TABLES_COUNT {
            let table = tables.add(table_index);
            let table_address = table as u32;
            (*directory).0[table_index] = table_address | PRESENT_WRITABLE | USER_PAGE;

            for page_index in 0..PAGE_ENTRIES {
                let address = (table_index * PAGE_ENTRIES + page_index) * PAGE_SIZE;
                let user = if address >= USER_MEMORY_BASE { USER_PAGE } else { 0 };
                (*table).0[page_index] = address as u32 | PRESENT_WRITABLE | user;
            }
        }

        core::arch::asm!("mov cr3, {0:e}", in(reg) directory as u32, options(nostack, preserves_flags));
        core::arch::asm!(
            "mov eax, cr0",
            "or eax, 0x80010000",
            "mov cr0, eax",
            out("eax") _,
            options(nostack, preserves_flags)
        );
    }
}

pub fn user_range_is_valid(address: u32, length: usize) -> bool {
    let start = address as usize;
    start >= USER_MEMORY_BASE
        && start < USER_MEMORY_END
        && length <= USER_MEMORY_END - start
}

const PAGE_TABLES_COUNT: usize = IDENTITY_SIZE / (PAGE_SIZE * PAGE_ENTRIES);

const fn size_of_page() -> usize {
    PAGE_SIZE
}
