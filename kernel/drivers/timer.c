#include "drivers/timer.h"
#include "arch/isr.h"
#include "arch/io.h"
#include "arch/pic.h"

#define PIT_COMMAND_PORT 0x43
#define PIT_DATA_PORT_0  0x40
#define PIT_BASE_FREQ    1193182

static volatile uint32_t timer_ticks = 0;
static uint32_t timer_frequency = PIT_DEFAULT_HZ;

static void timer_callback(registers_t *regs) {
    (void)regs;
    timer_ticks++;
}

void timer_init(uint32_t frequency) {
    timer_frequency = frequency ? frequency : PIT_DEFAULT_HZ;
    uint32_t divisor = PIT_BASE_FREQ / timer_frequency;
    if (divisor > 65535) divisor = 65535;

    // Send command byte: Channel 0, lobyte/hibyte, Mode 3 (Square Wave), 16-bit binary
    outb(PIT_COMMAND_PORT, 0x36);
    outb(PIT_DATA_PORT_0, (uint8_t)(divisor & 0xFF));
    outb(PIT_DATA_PORT_0, (uint8_t)((divisor >> 8) & 0xFF));

    // Register IRQ0 handler (vector 32 = IRQ0)
    isr_install_handler(32, timer_callback);

    // Unmask IRQ0 on Master PIC
    pic_clear_mask(0);
}

uint32_t timer_get_ticks(void) {
    return timer_ticks;
}

uint32_t timer_get_uptime_seconds(void) {
    return timer_ticks / timer_frequency;
}

void timer_sleep(uint32_t ticks) {
    uint32_t target = timer_ticks + ticks;
    while (timer_ticks < target) {
        __asm__ volatile ("hlt");
    }
}
