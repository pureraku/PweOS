; PweOS - First Bootloader
; Architecture: x86, 16-bit real mode
; Boot format: 512-byte BIOS boot sector

BITS 16
ORG 0x7C00

start:
    ; Set up the CPU segment registers.
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Set up a stack below the bootloader.
    mov sp, 0x7C00

    ; Restore normal interrupt operation.
    sti
    cld

    ; BIOS teletype output.
    mov si, message

print_char:
    lodsb
    test al, al
    jz halt

    mov ah, 0x0E
    int 0x10

    jmp print_char

halt:
    cli

.stop:
    hlt
    jmp .stop

message:
    db 13, 10, 'Welcome to PweOS!', 13, 10, 0

; Pad to exactly 512 bytes.
times 510 - ($ - $$) db 0

; BIOS boot signature.
dw 0xAA55
