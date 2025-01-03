#[macro_export]
macro_rules! export_symbol_simple {
    ($func:ident) => {
        core::arch::global_asm!(
            ".section \"__ksymtab_strings\",\"aMS\",%progbits,1",
            concat!("__kstrtab_", stringify!($func), ":"),
            concat!(".asciz \"", stringify!($func), "\""),
            ".previous",
            ".section \"___ksymtab\", \"a\"",
            ".balign 8",
            concat!("__ksymtab_", stringify!($func), ":"),
            concat!(".long ", stringify!($func)),
            concat!(".long __kstrtab_", stringify!($func)),
            ".previous"
        );
    };
}

#[macro_export]
macro_rules! export_func_simple {
    ($func:ident) => {
        crate::export_symbol_simple!($func);
    }
}

#[macro_export]
macro_rules! export_data_simple {
    ($func:ident) => {
        crate::export_symbol_simple!($func);
    }
}