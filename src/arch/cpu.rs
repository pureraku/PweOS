use core::arch::asm;

#[derive(Clone)]
pub struct CpuInfo {
    pub vendor: [u8; 13],
    pub brand: [u8; 49],
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    pub has_fpu: bool,
    pub has_tsc: bool,
    pub has_msr: bool,
    pub has_apic: bool,
    pub has_sse: bool,
    pub has_sse2: bool,
    pub has_hypervisor: bool,
}

impl CpuInfo {
    pub const fn new() -> Self {
        Self {
            vendor: [0; 13],
            brand: [0; 49],
            family: 0,
            model: 0,
            stepping: 0,
            has_fpu: false,
            has_tsc: false,
            has_msr: false,
            has_apic: false,
            has_sse: false,
            has_sse2: false,
            has_hypervisor: false,
        }
    }

    pub fn vendor_str(&self) -> &str {
        core::str::from_utf8(&self.vendor)
            .unwrap_or("")
            .trim_end_matches('\0')
    }

    pub fn brand_str(&self) -> &str {
        core::str::from_utf8(&self.brand)
            .unwrap_or("")
            .trim_end_matches('\0')
    }
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self::new()
    }
}

pub fn cpuid(leaf: u32, sub_leaf: u32) -> (u32, u32, u32, u32) {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;
    unsafe {
        asm!(
            "cpuid",
            inout("eax") leaf => eax,
            inout("ecx") sub_leaf => ecx,
            lateout("ebx") ebx,
            lateout("edx") edx,
            options(nomem, nostack, preserves_flags)
        );
    }
    (eax, ebx, ecx, edx)
}

pub fn cpu_get_info() -> CpuInfo {
    let mut info = CpuInfo::new();

    // Leaf 0: Vendor ID
    let (max_leaf, b, c, d) = cpuid(0, 0);

    info.vendor[0..4].copy_from_slice(&b.to_le_bytes());
    info.vendor[4..8].copy_from_slice(&d.to_le_bytes());
    info.vendor[8..12].copy_from_slice(&c.to_le_bytes());
    info.vendor[12] = 0;

    // Leaf 1: Family, Model, Stepping and Features
    if max_leaf >= 1 {
        let (eax1, _ebx1, ecx1, edx1) = cpuid(1, 0);

        info.stepping = eax1 & 0x0F;
        info.model = (eax1 >> 4) & 0x0F;
        info.family = (eax1 >> 8) & 0x0F;

        if info.family == 0x0F {
            info.family += (eax1 >> 20) & 0xFF;
        }
        if info.family == 0x06 || info.family == 0x0F {
            info.model += ((eax1 >> 16) & 0x0F) << 4;
        }

        info.has_fpu = (edx1 & (1 << 0)) != 0;
        info.has_tsc = (edx1 & (1 << 4)) != 0;
        info.has_msr = (edx1 & (1 << 5)) != 0;
        info.has_apic = (edx1 & (1 << 9)) != 0;
        info.has_sse = (edx1 & (1 << 25)) != 0;
        info.has_sse2 = (edx1 & (1 << 26)) != 0;
        info.has_hypervisor = (ecx1 & (1 << 31)) != 0;
    }

    // Extended Leaf 0x80000000: Brand string
    let (max_ext, _, _, _) = cpuid(0x80000000, 0);

    if max_ext >= 0x80000004 {
        let mut brand_buf = [0u8; 48];
        let (a2, b2, c2, d2) = cpuid(0x80000002, 0);
        brand_buf[0..4].copy_from_slice(&a2.to_le_bytes());
        brand_buf[4..8].copy_from_slice(&b2.to_le_bytes());
        brand_buf[8..12].copy_from_slice(&c2.to_le_bytes());
        brand_buf[12..16].copy_from_slice(&d2.to_le_bytes());

        let (a3, b3, c3, d3) = cpuid(0x80000003, 0);
        brand_buf[16..20].copy_from_slice(&a3.to_le_bytes());
        brand_buf[20..24].copy_from_slice(&b3.to_le_bytes());
        brand_buf[24..28].copy_from_slice(&c3.to_le_bytes());
        brand_buf[28..32].copy_from_slice(&d3.to_le_bytes());

        let (a4, b4, c4, d4) = cpuid(0x80000004, 0);
        brand_buf[32..36].copy_from_slice(&a4.to_le_bytes());
        brand_buf[36..40].copy_from_slice(&b4.to_le_bytes());
        brand_buf[40..44].copy_from_slice(&c4.to_le_bytes());
        brand_buf[44..48].copy_from_slice(&d4.to_le_bytes());

        info.brand[0..48].copy_from_slice(&brand_buf);
        info.brand[48] = 0;
    } else {
        let default_brand = b"Unknown x86 CPU";
        info.brand[0..default_brand.len()].copy_from_slice(default_brand);
        info.brand[default_brand.len()] = 0;
    }

    info
}
