; ==============================================================================
; PweOS - Stage 1 BIOS Bootloader
; Architecture: x86 (16-bit real mode -> 32-bit protected mode)
; ==============================================================================

BITS 16
ORG 0x7C00

KERNEL_LOAD_SEG  equ 0x1000      ; Physical address 0x10000
KERNEL_LOAD_OFF  equ 0x0000
KERNEL_SECTORS   equ 512         ; Read up to 512 sectors (256 KB)

start:
    ; Set up segment registers and stack in real mode
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    cld

    ; Save BIOS boot drive number
    mov [boot_drive], dl

    ; Display loading message
    mov si, msg_loading
    call print_string

    ; Load kernel sectors from disk to KERNEL_LOAD_SEG:KERNEL_LOAD_OFF
    call load_kernel

    ; Enable A20 address line
    call enable_a20

    ; Enter 32-bit protected mode
    cli
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; Far jump to flush prefetch queue and set CS to code segment selector (0x08)
    jmp CODE_SEG:init_protected_mode

; ------------------------------------------------------------------------------
; Real mode print string routine (via BIOS int 0x10)
; ------------------------------------------------------------------------------
print_string:
    push ax
    push bx
.loop:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    mov bx, 0x0007
    int 0x10
    jmp .loop
.done:
    pop bx
    pop ax
    ret

; ------------------------------------------------------------------------------
; Disk reading routine: loads KERNEL_SECTORS starting from sector LBA 1
; ------------------------------------------------------------------------------
load_kernel:
    mov ax, KERNEL_LOAD_SEG
    mov es, ax
    mov bx, KERNEL_LOAD_OFF     ; Destination ES:BX = 0x1000:0x0000 (0x10000)
    mov si, 1                   ; Starting LBA: sector 1 (0-indexed, sector 2 on floppy)
    mov di, KERNEL_SECTORS

.read_loop:
    test di, di
    jz .read_done

    ; Convert LBA in SI to CHS
    mov ax, si
    call lba_to_chs

    ; Issue BIOS Read Sectors: AH=0x02, AL=1 sector
    mov ax, 0x0201
    mov dl, [boot_drive]
    int 0x13
    jnc .read_ok

    ; Reset disk system on failure and retry once
    xor ax, ax
    mov dl, [boot_drive]
    int 0x13

    mov ax, si
    call lba_to_chs
    mov ax, 0x0201
    mov dl, [boot_drive]
    int 0x13
    jc disk_error

.read_ok:
    mov ax, es
    add ax, 0x20
    mov es, ax
    inc si
    dec di
    jmp .read_loop

.read_done:
    ret

disk_error:
    mov si, msg_disk_error
    call print_string
.halt:
    hlt
    jmp .halt

; ------------------------------------------------------------------------------
; LBA to CHS conversion for 1.44M Floppy (18 sectors/track, 2 heads)
; Input: AX = LBA
; Output: CH = Cylinder, DH = Head, CL = Sector (1-based)
; ------------------------------------------------------------------------------
lba_to_chs:
    push bx
    xor dx, dx
    mov bx, 18                  ; Sectors per track
    div bx                      ; AX = LBA / 18, DX = LBA % 18
    inc dx                      ; Sector is 1-based (1..18)
    mov cl, dl                  ; CL = Sector number

    xor dx, dx
    mov bx, 2                   ; Number of heads
    div bx                      ; AX = Cylinder, DX = Head
    mov ch, al                  ; CH = Cylinder
    mov dh, dl                  ; DH = Head
    pop bx
    ret

; ------------------------------------------------------------------------------
; Enable A20 Line via Fast A20 gate
; ------------------------------------------------------------------------------
enable_a20:
    in al, 0x92
    test al, 2
    jnz .done
    or al, 2
    and al, 0xFE                ; Do not trigger fast reset (bit 0)
    out 0x92, al
.done:
    ret

; ------------------------------------------------------------------------------
; Global Descriptor Table (GDT) for 32-bit flat memory model
; ------------------------------------------------------------------------------
align 4
gdt_start:
    ; Null descriptor (offset 0x00)
    dd 0x00000000
    dd 0x00000000

    ; Code segment descriptor (offset 0x08): Base=0, Limit=4GB, Type=0x9A, Flags=0xC
    dw 0xFFFF                   ; Limit 15:0
    dw 0x0000                   ; Base 15:0
    db 0x00                     ; Base 23:16
    db 10011010b                ; Present, Ring 0, Code, Executable, Readable
    db 11001111b                ; Granularity (4K), 32-bit size, Limit 19:16
    db 0x00                     ; Base 31:24

    ; Data segment descriptor (offset 0x10): Base=0, Limit=4GB, Type=0x92, Flags=0xC
    dw 0xFFFF                   ; Limit 15:0
    dw 0x0000                   ; Base 15:0
    db 0x00                     ; Base 23:16
    db 10010010b                ; Present, Ring 0, Data, Writable
    db 11001111b                ; Granularity (4K), 32-bit size, Limit 19:16
    db 0x00                     ; Base 31:24
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1   ; Limit
    dd gdt_start                 ; Base

CODE_SEG equ gdt_start + 0x08 - gdt_start   ; 0x08
DATA_SEG equ gdt_start + 0x10 - gdt_start   ; 0x10

; ------------------------------------------------------------------------------
; 32-Bit Protected Mode Entry Point
; ------------------------------------------------------------------------------
BITS 32
init_protected_mode:
    ; Set segment registers to point to 32-bit flat data segment
    mov ax, DATA_SEG
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Initialize stack pointer safely below video memory
    mov esp, 0x90000

    ; Jump to kernel entry point at physical address 0x10000
    jmp 0x10000

; ------------------------------------------------------------------------------
; Data & Boot Signature
; ------------------------------------------------------------------------------
BITS 16
boot_drive:      db 0
msg_loading:     db "Loading PweOS kernel...", 13, 10, 0
msg_disk_error:  db "Disk error loading kernel!", 13, 10, 0

; Pad to 510 bytes and add BIOS magic number
times 510 - ($ - $$) db 0
dw 0xAA55
