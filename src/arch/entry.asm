; ==============================================================================
; PweOS - 32-bit Kernel Entry
; ==============================================================================

[bits 32]

global _start
extern kmain

section .text.entry
_start:
    ; Ensure segment registers and stack are set
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000

    ; Call kernel main
    call kmain

    ; Halt if kmain returns
.halt:
    cli
    hlt
    jmp .halt
