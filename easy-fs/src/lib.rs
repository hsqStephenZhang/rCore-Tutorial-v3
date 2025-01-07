#![no_std]

extern crate alloc;

#[macro_use]
extern crate static_assertions;

pub mod bitmap;
pub mod block_cache;
pub mod block_dev;
pub mod efs;
pub mod layout;
pub mod vfs;
pub mod consts;
pub mod helpers;

pub(crate) use consts::*;
pub use block_dev::BlockDevice;
pub use bitmap::*;