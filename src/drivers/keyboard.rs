use super::serial::{serial_getchar, serial_has_byte};
use super::vga::vga_putchar;
use crate::arch::io::inb;
use crate::arch::isr::{isr_install_handler, Registers};
use crate::arch::pic::{pic_clear_mask, pic_set_mask};
use crate::sync::Spinlock;

pub const KEY_UP: u8 = 0x81;
pub const KEY_DOWN: u8 = 0x82;
pub const KEY_LEFT: u8 = 0x83;
pub const KEY_RIGHT: u8 = 0x84;
pub const KEY_DELETE: u8 = 0x85;

pub const KEYBOARD_DATA_PORT: u16 = 0x60;
pub const KEYBOARD_STATUS_PORT: u16 = 0x64;

const KB_BUFFER_SIZE: usize = 128;

struct KeyboardState {
    buffer: [u8; KB_BUFFER_SIZE],
    head: usize,
    tail: usize,
    shift_pressed: bool,
    caps_lock: bool,
    ctrl_pressed: bool,
    extended_prefix: bool,
}

impl KeyboardState {
    const fn new() -> Self {
        Self {
            buffer: [0; KB_BUFFER_SIZE],
            head: 0,
            tail: 0,
            shift_pressed: false,
            caps_lock: false,
            ctrl_pressed: false,
            extended_prefix: false,
        }
    }

    fn push(&mut self, c: u8) {
        let next_head = (self.head + 1) % KB_BUFFER_SIZE;
        if next_head != self.tail {
            self.buffer[self.head] = c;
            self.head = next_head;
        }
    }

    fn pop(&mut self) -> Option<u8> {
        if self.head != self.tail {
            let c = self.buffer[self.tail];
            self.tail = (self.tail + 1) % KB_BUFFER_SIZE;
            Some(c)
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }
}

static KB_STATE: Spinlock<KeyboardState> = Spinlock::new(KeyboardState::new());

const fn make_scancode_nomod_table() -> [u8; 128] {
    let mut table = [0u8; 128];
    table[1] = 27;
    table[2] = b'1'; table[3] = b'2'; table[4] = b'3'; table[5] = b'4'; table[6] = b'5';
    table[7] = b'6'; table[8] = b'7'; table[9] = b'8'; table[10] = b'9'; table[11] = b'0';
    table[12] = b'-'; table[13] = b'='; table[14] = b'\x08'; table[15] = b'\t';
    table[16] = b'q'; table[17] = b'w'; table[18] = b'e'; table[19] = b'r'; table[20] = b't';
    table[21] = b'y'; table[22] = b'u'; table[23] = b'i'; table[24] = b'o'; table[25] = b'p';
    table[26] = b'['; table[27] = b']'; table[28] = b'\n';
    table[30] = b'a'; table[31] = b's'; table[32] = b'd'; table[33] = b'f'; table[34] = b'g';
    table[35] = b'h'; table[36] = b'j'; table[37] = b'k'; table[38] = b'l'; table[39] = b';';
    table[40] = b'\''; table[41] = b'`';
    table[43] = b'\\'; table[44] = b'z'; table[45] = b'x'; table[46] = b'c'; table[47] = b'v';
    table[48] = b'b'; table[49] = b'n'; table[50] = b'm'; table[51] = b','; table[52] = b'.';
    table[53] = b'/'; table[55] = b'*'; table[57] = b' ';
    table
}

const fn make_scancode_shift_table() -> [u8; 128] {
    let mut table = [0u8; 128];
    table[1] = 27;
    table[2] = b'!'; table[3] = b'@'; table[4] = b'#'; table[5] = b'$'; table[6] = b'%';
    table[7] = b'^'; table[8] = b'&'; table[9] = b'*'; table[10] = b'('; table[11] = b')';
    table[12] = b'_'; table[13] = b'+'; table[14] = b'\x08'; table[15] = b'\t';
    table[16] = b'Q'; table[17] = b'W'; table[18] = b'E'; table[19] = b'R'; table[20] = b'T';
    table[21] = b'Y'; table[22] = b'U'; table[23] = b'I'; table[24] = b'O'; table[25] = b'P';
    table[26] = b'{'; table[27] = b'}'; table[28] = b'\n';
    table[30] = b'A'; table[31] = b'S'; table[32] = b'D'; table[33] = b'F'; table[34] = b'G';
    table[35] = b'H'; table[36] = b'J'; table[37] = b'K'; table[38] = b'L'; table[39] = b':';
    table[40] = b'"'; table[41] = b'~';
    table[43] = b'|'; table[44] = b'Z'; table[45] = b'X'; table[46] = b'C'; table[47] = b'V';
    table[48] = b'B'; table[49] = b'N'; table[50] = b'M'; table[51] = b'<'; table[52] = b'>';
    table[53] = b'?'; table[55] = b'*'; table[57] = b' ';
    table
}

static SCANCODE_ASCII_NOMOD: [u8; 128] = make_scancode_nomod_table();
static SCANCODE_ASCII_SHIFT: [u8; 128] = make_scancode_shift_table();

fn keyboard_callback(_regs: &mut Registers) {
    let scancode = unsafe { inb(KEYBOARD_DATA_PORT) };
    let mut state = KB_STATE.lock();

    if scancode == 0xE0 {
        state.extended_prefix = true;
        return;
    }

    if state.extended_prefix {
        state.extended_prefix = false;
        let is_release = (scancode & 0x80) != 0;
        let code = scancode & 0x7F;

        if code == 0x1D {
            // Right Ctrl
            state.ctrl_pressed = !is_release;
            return;
        }

        if is_release {
            return;
        }

        if scancode == 0x48 {
            state.push(KEY_UP);
        } else if scancode == 0x50 {
            state.push(KEY_DOWN);
        } else if scancode == 0x4B {
            state.push(KEY_LEFT);
        } else if scancode == 0x4D {
            state.push(KEY_RIGHT);
        } else if scancode == 0x53 {
            state.push(KEY_DELETE);
        }
        return;
    }

    // Key release (break code)
    if (scancode & 0x80) != 0 {
        let released = scancode & 0x7F;
        if released == 0x2A || released == 0x36 {
            state.shift_pressed = false;
        }
        if released == 0x1D {
            // Left Ctrl released
            state.ctrl_pressed = false;
        }
        return;
    }

    // Key press (make code)
    if scancode == 0x2A || scancode == 0x36 {
        state.shift_pressed = true;
        return;
    }

    if scancode == 0x1D {
        // Left Ctrl pressed
        state.ctrl_pressed = true;
        return;
    }

    if scancode == 0x3A {
        state.caps_lock = !state.caps_lock;
        return;
    }

    if (scancode as usize) < 128 {
        let idx = scancode as usize;
        let c_normal = SCANCODE_ASCII_NOMOD[idx];
        let c_shift = SCANCODE_ASCII_SHIFT[idx];

        let c = if c_normal.is_ascii_lowercase() {
            let uppercase = state.shift_pressed ^ state.caps_lock;
            if uppercase {
                c_shift
            } else {
                c_normal
            }
        } else if state.shift_pressed {
            c_shift
        } else {
            c_normal
        };

        if c != 0 {
            if state.ctrl_pressed && c.is_ascii_alphabetic() {
                // Ctrl+letter → control character (e.g. Ctrl+L = 0x0C)
                state.push(c.to_ascii_uppercase() & 0x1F);
            } else {
                state.push(c);
            }
        }
    }
}

pub fn keyboard_init() {
    {
        let mut state = KB_STATE.lock();
        state.head = 0;
        state.tail = 0;
        state.shift_pressed = false;
        state.caps_lock = false;
        state.ctrl_pressed = false;
        state.extended_prefix = false;
    }

    // Register IRQ1 handler (vector 33)
    isr_install_handler(33, keyboard_callback);

    // Unmask IRQ1 on Master PIC
    unsafe {
        pic_clear_mask(1);
    }
}

pub fn keyboard_has_char() -> bool {
    if !KB_STATE.lock().is_empty() {
        return true;
    }
    serial_has_byte()
}

pub fn keyboard_getchar() -> u8 {
    loop {
        // First check serial port
        if serial_has_byte() {
            let mut c = serial_getchar();
            if c == 0x1B {
                // ANSI escape sequence
                if serial_has_byte() {
                    let next = serial_getchar();
                    if next == b'[' && serial_has_byte() {
                        let code = serial_getchar();
                        if code == b'A' {
                            return KEY_UP;
                        }
                        if code == b'B' {
                            return KEY_DOWN;
                        }
                        if code == b'C' {
                            return KEY_RIGHT;
                        }
                        if code == b'D' {
                            return KEY_LEFT;
                        }
                        if code == b'3' && serial_has_byte() && serial_getchar() == b'~' {
                            return KEY_DELETE;
                        }
                    }
                }
            }
            if c == b'\r' {
                c = b'\n';
            }
            if c == 127 {
                c = b'\x08'; // Map DEL to backspace
            }
            return c;
        }

        // Next check keyboard buffer
        if let Some(c) = KB_STATE.lock().pop() {
            return c;
        }

        // Wait for interrupt
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Wait for a physical keypress while reading PS/2 make codes directly.
/// This is useful for full-screen views where translated shell input is paused.
pub fn keyboard_wait_for_keypress() {
    unsafe { pic_set_mask(1) };
    loop {
        if serial_has_byte() {
            let _ = serial_getchar();
            break;
        }
        if unsafe { inb(KEYBOARD_STATUS_PORT) } & 0x01 != 0 {
            let scancode = unsafe { inb(KEYBOARD_DATA_PORT) };
            if scancode & 0x80 == 0 {
                break;
            }
        }
        unsafe { core::arch::asm!("hlt") };
    }
    unsafe { pic_clear_mask(1) };
}

pub fn keyboard_readline(buffer: &mut [u8]) -> usize {
    if buffer.is_empty() {
        return 0;
    }

    let mut index = 0;
    let max_len = buffer.len();

    while index < max_len - 1 {
        let c = keyboard_getchar();

        if c == b'\n' || c == b'\r' {
            vga_putchar(b'\n');
            buffer[index] = 0;
            return index;
        } else if c == b'\x08' {
            if index > 0 {
                index -= 1;
                vga_putchar(b'\x08');
            }
        } else if (32..=126).contains(&c) {
            buffer[index] = c;
            index += 1;
            vga_putchar(c);
        }
    }

    buffer[index] = 0;
    vga_putchar(b'\n');
    index
}
