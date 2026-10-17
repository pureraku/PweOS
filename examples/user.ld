ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS
{
    . = 0x00400000;
    .text : ALIGN(4096) { *(.text.entry) *(.text*) } :text
    .rodata : ALIGN(16) { *(.rodata*) } :text
    . = ALIGN(4096);
    .data : { *(.data*) } :data
    .bss : { *(COMMON) *(.bss*) } :data

    /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame*) }
}
