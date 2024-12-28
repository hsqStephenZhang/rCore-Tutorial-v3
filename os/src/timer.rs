//! RISC-V timer-related functionality

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

/// read the `mtime` register: clock cycles
pub fn get_time() -> usize {
    time::read()
}

/// get current time in milliseconds
/// divide by `CLOCK_FREQ` to convert clock cycles to seconds
/// multiply by `MSEC_PER_SEC` to convert seconds to milliseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

/// set the next timer interrupt
/// duration is 1 sec
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
