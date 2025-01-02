
        .section "__ksymtab_strings","aMS",%progbits,1
        __kstrtab_trap_handler:
            .asciz "trap_handler"
        .previous
        .section "___ksymtab", "a"
        .balign 8
        __ksymtab_trap_handler:
            .long trap_handler
            .long __kstrtab_trap_handler
        .previous
        

        .section "__ksymtab_strings","aMS",%progbits,1
        __kstrtab_rust_main:
            .asciz "rust_main"
        .previous
        .section "___ksymtab", "a"
        .balign 8
        __ksymtab_rust_main:
            .long rust_main
            .long __kstrtab_rust_main
        .previous
        

        .section "__ksymtab_strings","aMS",%progbits,1
        __kstrtab_do_initcalls:
            .asciz "do_initcalls"
        .previous
        .section "___ksymtab", "a"
        .balign 8
        __ksymtab_do_initcalls:
            .long do_initcalls
            .long __kstrtab_do_initcalls
        .previous
        
