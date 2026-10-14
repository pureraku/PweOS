/* Linker script for PweOS 32-bit x86 kernel */
ENTRY(_start)

SECTIONS
{
    . = 0x10000;

    .text ALIGN(4K) :
    {
        *(.text.entry)
        *(.text*)
    }

    .rodata ALIGN(4K) :
    {
        *(.rodata*)
    }

    .data ALIGN(4K) :
    {
        *(.data*)
    }

    .bss ALIGN(4K) :
    {
        *(COMMON)
        *(.bss*)
    }

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
        *(.eh_frame)
    }
}
