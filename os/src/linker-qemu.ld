OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(4K);
        *(.text .text.*)
    }
    . = ALIGN(4K);
    etext = .;

    . = ALIGN(4K);
    sinittext = .;
    .init.text : {
        *(.init.text .init.text.*)
    }
    einittext = .;

    . = ALIGN(4K);
    sinitdata = .;
    .init.data : {
        initcall_start = .;
        KEEP(*(.initcall.init))
        initcall1_start = .;
        KEEP(*(.initcall1.init))
        initcall2_start = .;
        KEEP(*(.initcall2.init))
        initcall3_start = .;
        KEEP(*(.initcall3.init))
        initcall4_start = .;
        KEEP(*(.initcall4.init))
        initcall5_start = .;
        KEEP(*(.initcall5.init))
        initcall6_start = .;
        KEEP(*(.initcall6.init))
        initcall7_start = .;
        KEEP(*(.initcall7.init))
        initcall_end = .;
    }
    einitdata = .;

    . = ALIGN(4K);
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    sbss_with_stack = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}