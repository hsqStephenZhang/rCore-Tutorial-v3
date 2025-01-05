//! Process management syscalls
use log::{info, trace};

use crate::mm::{copy_to_user, translate_user_str};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next, TaskStatus,
};
use crate::timer::get_time_us;

#[repr(C)]
pub struct TaskInfo {}

/// task exits and submit an exit code
#[no_mangle]
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code as isize);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
#[no_mangle]
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// get time in milliseconds
#[no_mangle]
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let usec = get_time_us();
    let sec = usec / 1_000_000;
    let usec = usec % 1_000_000;
    let time = TimeVal { sec, usec };

    let current_token = current_user_token();
    copy_to_user(current_token, &time, ts);
    0
}

#[no_mangle]
pub fn sys_task_info(_ptr: *mut TaskInfo) -> isize {
    println!("[kernel] sys_task_info is not implemented!");
    todo!()
}

/// 功能：当前进程 fork 出来一个子进程。
/// 返回值：对于子进程返回 0，对于当前进程则返回子进程的 PID 。
/// syscall ID：220
#[no_mangle]
pub fn sys_fork() -> isize {
    let task = current_task().unwrap();
    let new_task = task.fork();
    let pid = new_task.getpid();

    let inner = new_task.inner();
    inner.get_trap_cx().x[10] = 0;
    drop(inner);
    add_task(new_task);

    pid as _
}

/// 功能：将当前进程的地址空间清空并加载一个特定的可执行文件，返回用户态后开始它的执行。
/// 参数：path 给出了要加载的可执行文件的名字；
/// 返回值：如果出错的话（如找不到名字相符的可执行文件）则返回 -1，否则不应该返回。
/// syscall ID：221
#[no_mangle]
pub fn sys_exec(path: *const u8) -> isize {
    let task = current_task().unwrap();
    let current_user_token = current_user_token();
    let path = translate_user_str(current_user_token, path);
    info!("sys_exec: path = {:?}", path);
    match task.exec(&path) {
        Ok(_) => 0,
        Err(()) => -1,
    }
}

#[no_mangle]
pub fn sys_spawn(path: *const u8) -> isize {
    let parent = current_task().unwrap();
    let current_user_token = current_user_token();
    let path = translate_user_str(current_user_token, path);

    let child = parent.fork();
    if let Err(_) = child.exec(&path) {
        return -1;
    }
    let pid = child.getpid();

    info!(
        "sys_spawn, running child {:?}, pid: {}",
        child.get_cmdline(),
        pid
    );
    add_task(child);

    pid as _
}

#[repr(C)]
pub struct WaitPidOption {
    // 0: block, 1: non-block
    pub nowait: bool,
}

/// 功能：当前进程等待一个子进程变为僵尸进程，回收其全部资源并收集其返回值。
/// 参数：pid 表示要等待的子进程的进程 ID，如果为 -1 的话表示等待任意一个子进程；
/// exit_code 表示保存子进程返回值的地址，如果这个地址为 0 的话表示不必保存。
/// 返回值：如果要等待的子进程不存在则返回 -1；否则如果要等待的子进程均未结束则返回 -2；
/// 否则返回结束的子进程的进程 ID。
/// syscall ID：260
#[no_mangle]
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32, opt: WaitPidOption) -> isize {
    if !opt.nowait {
        println!("[kernel] sys_waitpid does not support wait queue!");
        todo!()
    }
    let current_token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner();

    if inner
        .children
        .iter()
        .find(|p| pid == -1 || pid as usize == p.getpid())
        .is_none()
    {
        trace!(
            "[kernel] sys_waitpid in {:?}: no child found, children len: {}",
            inner.cmdline.as_ref(),
            inner.children.len()
        );
        return -1;
        // ---- stop exclusively accessing current PCB
    }

    let res = inner
        .children
        .iter()
        .enumerate()
        .find(|(_, t)| {
            if t.status() == TaskStatus::Zombie && (pid == -1 || pid as usize == t.getpid()) {
                return true;
            }
            false
        })
        .map(|(idx, t)| (idx, t.clone()));

    if let Some((idx, child)) = res {
        let task_pid = child.getpid();
        let cmdline = child.get_cmdline();
        trace!(
            "[kernel] sys_waitpid in {:?}: found child {:?} to wait, target pid: {}, actual pid: {}",
            inner.cmdline.as_ref(),
            cmdline,
            pid,task_pid
        );
        // TODO: copy exit code
        let exit_code = child.exit_code() as i32;
        copy_to_user(current_token, &exit_code, exit_code_ptr);
        inner.children.remove(idx);
        task_pid as isize
    } else {
        -2
    }
}

#[no_mangle]
pub fn sys_getpid() -> isize {
    current_task().unwrap().getpid() as isize
}

export_func_simple!(sys_exit);
export_func_simple!(sys_yield);
export_func_simple!(sys_get_time);
export_func_simple!(sys_task_info);
export_func_simple!(sys_fork);
export_func_simple!(sys_exec);
export_func_simple!(sys_waitpid);
export_func_simple!(sys_getpid);
export_data_simple!(sys_spawn);
