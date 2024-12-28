use std::{
    fs::{read_dir, File},
    io::Write,
};

fn main() {
    println!("cargo:rerun-if-changed=../user/src");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_user_bin().unwrap();
}

const TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release";

fn insert_user_bin() -> Result<(), std::io::Error> {
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

    let num_app = apps.len();

    writeln!(
        f,
        r#"    .align 3
    .section .data
    .global _num_app
_num_app:"#,
    )?;
    writeln!(f, "    .quad {}", num_app)?;
    for i in 0..num_app {
        writeln!(f, "    .quad app_{}_start", i)?;
    }
    writeln!(f, "    .quad app_{}_end", num_app - 1)?;
    writeln!(f)?;

    for i in 0..num_app {
        writeln!(
            f,
            r#"    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/{1}.bin"
app_{0}_end:
"#,
            i, apps[i]
        )?;
    }

    writeln!(
        f,
        r#"
    // Add string literals for each app's name
    .section .rodata
    .global _app_names
_app_names:"#
    )?;

    for i in 0..num_app {
        writeln!(f, ".string \"{}\"", &apps[i])?;
    }

    Ok(())
}
