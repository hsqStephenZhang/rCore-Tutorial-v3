///! kernel will write all exported symbols into export_symbol.S by build.rs(the 
/// symbols list to be included is hard-coded in `build.rs` currently, it can be
/// improved by analyzing the elf file after the first stage of compiling, you may
/// refer to linux's approach in `scripts/kallsyms.c`)
/// then we can map this section into kernel's memory space, and read all the content
/// load it into a BTreeMap, and we can lookup the symbol by address
/// (then we may deallocate the memory of `__ksymtab_strings` and `___ksymtab` sections)
/// 

use alloc::{borrow::ToOwned, string::String};

extern "C" {
    fn s_ksymtab();
    fn e_ksymtab();
}

lazy_static::lazy_static! {
    pub static ref SYMBOLS: alloc::collections::BTreeMap<usize, String> = load_symbols();
}

#[repr(C)]
struct KSym {
    // actual symbol address of kernel
    addr: u32,
    // the address of the symbols's name in `__ksymtab_strings` section
    name: u32,
}

fn load_symbols() -> alloc::collections::BTreeMap<usize, String> {
    println!("Loading symbols");
    let ksymtab_start = s_ksymtab as usize;
    let ksymtab_end = e_ksymtab as usize;
    let ksymtab = unsafe {
        core::slice::from_raw_parts(
            ksymtab_start as *const KSym,
            (ksymtab_end - ksymtab_start) / core::mem::size_of::<KSym>(),
        )
    };
    println!("num symbols: {}", ksymtab.len());
    let mut symbols = alloc::collections::BTreeMap::new();
    for sym in ksymtab {
        let addr = sym.addr as usize;
        let name_start = sym.name as usize as *const u8;
        let name = unsafe {
            let mut len = 0;
            while *name_start.offset(len) != 0 {
                len += 1;
            }
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(name_start, len as _))
        };
        symbols.insert(addr, name.to_owned());
    }
    symbols
}

pub fn print_all_symbols() {
    for (addr, name) in SYMBOLS.iter() {
        println!("{:#x} {}", addr, name);
    }
}

// it's not precise at all, just a quick demo
pub fn lookup(addr: usize) -> Option<&'static str> {
    SYMBOLS.range(..=addr).next_back().map(|(_, name)| name.as_str())
}