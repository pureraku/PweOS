#ifndef ARCH_CPU_H
#define ARCH_CPU_H

#include <stdint.h>
#include <stdbool.h>

typedef struct {
    char vendor[13];
    char brand[49];
    uint32_t family;
    uint32_t model;
    uint32_t stepping;
    bool has_fpu;
    bool has_tsc;
    bool has_msr;
    bool has_apic;
    bool has_sse;
    bool has_sse2;
    bool has_hypervisor;
} cpu_info_t;

void cpu_get_info(cpu_info_t *info);

#endif // ARCH_CPU_H
