use super::idt::{idt_init, idt_set_gate};
use super::io::outb;
use super::pic::{pic_remap, pic_send_eoi, PIC1_DATA, PIC2_DATA};
use crate::drivers::vga::{VgaColor, VGA_WRITER};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Registers {
    pub ds: u32,
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,
    pub int_no: u32,
    pub err_code: u32,
    pub eip: u32,
    pub cs: u32,
    pub eflags: u32,
    pub useresp: u32,
    pub ss: u32,
}

pub type IsrHandler = fn(&mut Registers);

use crate::sync::Spinlock;

static INTERRUPT_HANDLERS: Spinlock<[Option<IsrHandler>; 256]> = Spinlock::new([None; 256]);

static EXCEPTION_MESSAGES: [&str; 32] = [
    "Division By Zero",
    "Debug Trap",
    "Non-Maskable Interrupt",
    "Breakpoint",
    "Into Detected Overflow",
    "Out of Bounds",
    "Invalid Opcode",
    "No Coprocessor",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Bad TSS",
    "Segment Not Present",
    "Stack Fault",
    "General Protection Fault",
    "Page Fault",
    "Unknown Interrupt",
    "Coprocessor Fault",
    "Alignment Check",
    "Machine Check",
    "SIMD Floating-Point Exception",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
];

extern "C" {
    fn isr0();  fn isr1();  fn isr2();  fn isr3();
    fn isr4();  fn isr5();  fn isr6();  fn isr7();
    fn isr8();  fn isr9();  fn isr10(); fn isr11();
    fn isr12(); fn isr13(); fn isr14(); fn isr15();
    fn isr16(); fn isr17(); fn isr18(); fn isr19();
    fn isr20(); fn isr21(); fn isr22(); fn isr23();
    fn isr24(); fn isr25(); fn isr26(); fn isr27();
    fn isr28(); fn isr29(); fn isr30(); fn isr31();

    fn irq0();  fn irq1();  fn irq2();  fn irq3();
    fn irq4();  fn irq5();  fn irq6();  fn irq7();
    fn irq8();  fn irq9();  fn irq10(); fn irq11();
    fn irq12(); fn irq13(); fn irq14(); fn irq15();
    fn syscall_stub();
}

pub fn isr_install_handler(n: u8, handler: IsrHandler) {
    INTERRUPT_HANDLERS.lock()[n as usize] = Some(handler);
}

pub fn isr_uninstall_handler(n: u8) {
    INTERRUPT_HANDLERS.lock()[n as usize] = None;
}

pub fn isr_init() {
    idt_init();

    let isr_stubs: [unsafe extern "C" fn(); 32] = [
        isr0,  isr1,  isr2,  isr3,  isr4,  isr5,  isr6,  isr7,
        isr8,  isr9,  isr10, isr11, isr12, isr13, isr14, isr15,
        isr16, isr17, isr18, isr19, isr20, isr21, isr22, isr23,
        isr24, isr25, isr26, isr27, isr28, isr29, isr30, isr31,
    ];

    for (i, stub) in isr_stubs.iter().enumerate() {
        idt_set_gate(i, *stub as *const () as u32, 0x08, 0x8E);
    }

    // Remap the PIC to vectors 32..47
    unsafe {
        pic_remap(0x20, 0x28);
    }

    let irq_stubs: [unsafe extern "C" fn(); 16] = [
        irq0,  irq1,  irq2,  irq3,  irq4,  irq5,  irq6,  irq7,
        irq8,  irq9,  irq10, irq11, irq12, irq13, irq14, irq15,
    ];

    for (i, stub) in irq_stubs.iter().enumerate() {
        idt_set_gate(32 + i, *stub as *const () as u32, 0x08, 0x8E);
    }

    // DPL 3 allows user programs to invoke the system call gate with int 0x80.
    idt_set_gate(0x80, syscall_stub as *const () as u32, 0x08, 0xEE);

    // Mask all IRQs until specific drivers unmask them
    unsafe {
        outb(PIC1_DATA, 0xFF);
        outb(PIC2_DATA, 0xFF);
    }
    INTERRUPT_HANDLERS.lock().fill(None);
}

#[no_mangle]
pub unsafe extern "C" fn isr_handler(regs: *mut Registers) {
    let r = &mut *regs;
    let int_no = r.int_no as usize;

    if r.cs & 3 == 3 {
        let message = EXCEPTION_MESSAGES.get(int_no).copied().unwrap_or("Unknown Exception");
        println!("User program fault: {} (vector {}, error 0x{:x})", message, int_no, r.err_code);
        crate::process::abort_current(-1);
    }

    if int_no < 256 {
        let handler = INTERRUPT_HANDLERS.lock()[int_no];
        if let Some(h) = handler {
            h(r);
            return;
        }
    }

    // Unhandled CPU exception: Kernel Panic
    VGA_WRITER.lock().set_color(VgaColor::White, VgaColor::Red);
    println!("\n*** KERNEL PANIC: CPU EXCEPTION 0x{:02x} ***", r.int_no);
    if r.int_no < 32 {
        println!(
            "Description: {} (Error code: 0x{:x})",
            EXCEPTION_MESSAGES[r.int_no as usize], r.err_code
        );
    }
    println!(
        "EIP: 0x{:08x}  CS: 0x{:04x}  EFLAGS: 0x{:08x}",
        r.eip, r.cs, r.eflags
    );
    println!(
        "EAX: 0x{:08x}  EBX: 0x{:08x}  ECX: 0x{:08x}  EDX: 0x{:08x}",
        r.eax, r.ebx, r.ecx, r.edx
    );
    println!(
        "ESI: 0x{:08x}  EDI: 0x{:08x}  EBP: 0x{:08x}  ESP: 0x{:08x}",
        r.esi, r.edi, r.ebp, r.esp
    );
    println!("System halted.");

    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn irq_handler(regs: *mut Registers) {
    let r = &mut *regs;
    let int_no = r.int_no as usize;

    if int_no < 256 {
        let handler = INTERRUPT_HANDLERS.lock()[int_no];
        if let Some(h) = handler {
            h(r);
        }
    }

    unsafe {
        pic_send_eoi((r.int_no - 32) as u8);
    }
}
