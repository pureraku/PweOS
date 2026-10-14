use core::arch::asm;

#[inline]
pub unsafe fn outb(port: u16, val: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nomem, nostack, preserves_flags)
    );
}

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") ret,
        options(nomem, nostack, preserves_flags)
    );
    ret
}

#[inline]
pub unsafe fn outw(port: u16, val: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") val,
        options(nomem, nostack, preserves_flags)
    );
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let ret: u16;
    asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") ret,
        options(nomem, nostack, preserves_flags)
    );
    ret
}

#[inline]
pub unsafe fn io_wait() {
    outb(0x80, 0);
}
