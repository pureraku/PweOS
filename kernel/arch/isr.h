#ifndef ARCH_ISR_H
#define ARCH_ISR_H

#include <stdint.h>

typedef struct registers {
    uint32_t ds;                                      // Data segment selector
    uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax;  // Pushed by pusha
    uint32_t int_no, err_code;                        // Interrupt number and error code
    uint32_t eip, cs, eflags, useresp, ss;            // Pushed by processor automatically
} registers_t;

typedef void (*isr_t)(registers_t *);

void isr_install_handler(uint8_t n, isr_t handler);
void isr_uninstall_handler(uint8_t n);

void isr_handler(registers_t *regs);
void irq_handler(registers_t *regs);

void isr_init(void);

#endif // ARCH_ISR_H
