#include "arch/cpu.h"
#include "lib/string.h"

static void cpuid(uint32_t leaf, uint32_t sub_leaf, uint32_t *eax, uint32_t *ebx, uint32_t *ecx, uint32_t *edx) {
    uint32_t a, b, c, d;
    __asm__ volatile (
        "cpuid"
        : "=a"(a), "=b"(b), "=c"(c), "=d"(d)
        : "a"(leaf), "c"(sub_leaf)
    );
    if (eax) *eax = a;
    if (ebx) *ebx = b;
    if (ecx) *ecx = c;
    if (edx) *edx = d;
}

void cpu_get_info(cpu_info_t *info) {
    if (!info) return;
    memset(info, 0, sizeof(cpu_info_t));

    // Leaf 0: Vendor ID
    uint32_t max_leaf = 0;
    uint32_t b = 0, c = 0, d = 0;
    cpuid(0, 0, &max_leaf, &b, &c, &d);

    *(uint32_t *)&info->vendor[0] = b;
    *(uint32_t *)&info->vendor[4] = d;
    *(uint32_t *)&info->vendor[8] = c;
    info->vendor[12] = '\0';

    // Leaf 1: Family, Model, Stepping and Features
    if (max_leaf >= 1) {
        uint32_t eax1 = 0, ebx1 = 0, ecx1 = 0, edx1 = 0;
        cpuid(1, 0, &eax1, &ebx1, &ecx1, &edx1);

        info->stepping = eax1 & 0x0F;
        info->model    = (eax1 >> 4) & 0x0F;
        info->family   = (eax1 >> 8) & 0x0F;

        if (info->family == 0x0F) {
            info->family += (eax1 >> 20) & 0xFF;
        }
        if (info->family == 0x06 || info->family == 0x0F) {
            info->model += ((eax1 >> 16) & 0x0F) << 4;
        }

        info->has_fpu        = (edx1 & (1 << 0))  != 0;
        info->has_tsc        = (edx1 & (1 << 4))  != 0;
        info->has_msr        = (edx1 & (1 << 5))  != 0;
        info->has_apic       = (edx1 & (1 << 9))  != 0;
        info->has_sse        = (edx1 & (1 << 25)) != 0;
        info->has_sse2       = (edx1 & (1 << 26)) != 0;
        info->has_hypervisor = (ecx1 & (1 << 31)) != 0;
    }

    // Extended Leaf 0x80000000: Brand string
    uint32_t max_ext = 0;
    cpuid(0x80000000, 0, &max_ext, NULL, NULL, NULL);

    if (max_ext >= 0x80000004) {
        uint32_t brand_buf[12];
        cpuid(0x80000002, 0, &brand_buf[0], &brand_buf[1], &brand_buf[2], &brand_buf[3]);
        cpuid(0x80000003, 0, &brand_buf[4], &brand_buf[5], &brand_buf[6], &brand_buf[7]);
        cpuid(0x80000004, 0, &brand_buf[8], &brand_buf[9], &brand_buf[10], &brand_buf[11]);
        memcpy(info->brand, brand_buf, 48);
        info->brand[48] = '\0';
    } else {
        strcpy(info->brand, "Unknown x86 CPU");
    }
}
