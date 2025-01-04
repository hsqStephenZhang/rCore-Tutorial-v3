use log::debug;

use crate::{
    config::PAGE_SIZE,
    task::{mmap_current_task, munmap_current_task, sbrk_current_task},
};

#[no_mangle]
pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start & PAGE_SIZE - 1 != 0 {
        return -1;
    }

    munmap_current_task(start, len)
}

/// * start: 需要映射的虚存起始地址，要求按页对齐
/// * len: 映射字节长度，可以为 0
/// * prot: 权限. 第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
/// possible errors
/// start 没有按页大小对齐
/// prot & !0x7 != 0 (prot 其余位必须为0)
/// prot & 0x7 = 0 (这样的内存无意义)
/// [start, start + len) 中存在已经被映射的页
/// 物理内存不足
#[no_mangle]
pub fn sys_mmap(start: usize, len: usize, protection: usize) -> isize {
    println!("sys_mmap: start = {}, len = {}, protection = {}", start, len, protection);
    if protection & !0x7 != 0 || protection & 0x7 == 0 {
        return -1;
    }
    if start & PAGE_SIZE - 1 != 0 {
        return -1;
    }

    mmap_current_task(start, len, protection)
}

#[no_mangle]
pub fn sys_sbrk(s: i32) -> isize {
    let res = sbrk_current_task(s);
    debug!("sys_sbrk: s = {}, res = {:?}", s, res);
    match res {
        Some(origin_brk) => origin_brk as _,
        None => -1,
    }
}
