use std::fs::{read_dir, File};
use std::io::{Result, Write};

fn main() {
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_app_data().unwrap();
    mock_export_symbols(&["trap_handler", "rust_main", "do_initcalls"]).unwrap();
}

static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.S").unwrap();
    let mut apps: Vec<_> = read_dir("../user/src/bin")
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    apps.sort();

    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#,
            idx, app, TARGET_PATH
        )?;
    }
    Ok(())
}

// mock certain symbols
fn mock_export_symbols(symbols: &[&str]) -> Result<()> {
    let mut f = File::create("src/export_symbol.S").unwrap();

    let mut write_one_symbol = |symbol: &str| -> Result<()> {
        writeln!(
            f,
            r#"
        .section "__ksymtab_strings","aMS",%progbits,1
        __kstrtab_{0}:
            .asciz "{0}"
        .previous
        .section "___ksymtab", "a"
        .balign 8
        __ksymtab_{0}:
            .long {0}
            .long __kstrtab_{0}
        .previous
        "#,
            symbol
        )
    };
    for symbol in symbols.iter() {
        write_one_symbol(symbol)?;
    }

    Ok(())
}
