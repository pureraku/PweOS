use crate::arch::io::outb;
use crate::arch::isr::{isr_install_handler, Registers};
use crate::arch::pic::pic_clear_mask;
use core::sync::atomic::{AtomicU32, Ordering};

pub const PIT_COMMAND_PORT: u16 = 0x43;
pub const PIT_DATA_PORT_0: u16 = 0x40;
pub const PIT_BASE_FREQ: u32 = 1193182;
pub const PIT_DEFAULT_HZ: u32 = 100;

static TIMER_TICKS: AtomicU32 = AtomicU32::new(0);
static TIMER_FREQUENCY: AtomicU32 = AtomicU32::new(PIT_DEFAULT_HZ);

fn timer_callback(_regs: &mut Registers) {
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn timer_init(frequency: u32) {
    let freq = if frequency > 0 { frequency } else { PIT_DEFAULT_HZ };
    TIMER_FREQUENCY.store(freq, Ordering::Relaxed);

    let mut divisor = PIT_BASE_FREQ / freq;
    if divisor > 65535 {
        divisor = 65535;
    }

    unsafe {
        // Channel 0, lobyte/hibyte, Mode 3 (Square Wave), 16-bit binary
        outb(PIT_COMMAND_PORT, 0x36);
        outb(PIT_DATA_PORT_0, (divisor & 0xFF) as u8);
        outb(PIT_DATA_PORT_0, ((divisor >> 8) & 0xFF) as u8);

        // Vector 32 is IRQ0
        isr_install_handler(32, timer_callback);

        // Unmask IRQ0 on Master PIC
        pic_clear_mask(0);
    }
}

pub fn timer_get_ticks() -> u32 {
    TIMER_TICKS.load(Ordering::Relaxed)
}

pub fn timer_get_uptime_seconds() -> u32 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    timer_get_ticks().checked_div(freq).unwrap_or(0)
}

pub fn timer_sleep(ticks: u32) {
    let target = timer_get_ticks() + ticks;
    while timer_get_ticks() < target {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
