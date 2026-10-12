#include "arch/idt.h"
#include "lib/string.h"

extern void idt_flush(uint32_t);

static struct idt_entry idt_entries[IDT_ENTRIES];
static struct idt_ptr   idt_pointer;

void idt_set_gate(uint8_t num, uint32_t base, uint16_t sel, uint8_t flags) {
    idt_entries[num].base_low  = (uint16_t)(base & 0xFFFF);
    idt_entries[num].base_high = (uint16_t)((base >> 16) & 0xFFFF);
    idt_entries[num].sel       = sel;
    idt_entries[num].always0   = 0;
    idt_entries[num].flags     = flags;
}

void idt_init(void) {
    idt_pointer.limit = (uint16_t)(sizeof(struct idt_entry) * IDT_ENTRIES - 1);
    idt_pointer.base  = (uint32_t)&idt_entries;

    memset(&idt_entries, 0, sizeof(struct idt_entry) * IDT_ENTRIES);

    idt_flush((uint32_t)&idt_pointer);
}
