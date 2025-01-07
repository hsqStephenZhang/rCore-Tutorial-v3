use core::fmt;

use alloc::sync::Arc;
use log::debug;
use spin::{Mutex, MutexGuard};

use crate::{
    block_cache::{force_sync_all_cache, get_block_cache},
    layout::{DiskInode, SuperBlock, BLOCK_INODE_NUM},
    vfs::Inode,
    BitMapAlloc, Bitmap, BlockDevice, BLOCK_NUM_BITS, BLOCK_NUM_BYTES, ROOT_INODE_ID,
};

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}

impl fmt::Debug for EasyFileSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EasyFileSystem")
            .field("inode_bitmap", &self.inode_bitmap)
            .field("data_bitmap", &self.data_bitmap)
            .field("inode_area_start_block", &self.inode_area_start_block)
            .field("data_area_start_block", &self.data_area_start_block)
            .finish()
    }
}

impl EasyFileSystem {
    pub fn new(
        device: Arc<dyn BlockDevice>,
        total_blocks: usize,
        inode_bitmap_blocks: usize,
    ) -> Self {
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks);
        let inode_area_start_block = inode_bitmap_blocks as u32 + 1;
        let inode_area_num_blocks =
            inode_bitmap.max_inodes() * core::mem::size_of::<DiskInode>() / BLOCK_NUM_BYTES;
        let data_bitmap_start_block = inode_area_start_block + inode_area_num_blocks as u32;
        // let data_bitmap = Bitmap::new(inode_area_start_block as usize + inode_area_num_blocks as usize, 1);

        // calculate the number of blocks needed for the data bitmap
        let left_blocks = total_blocks - data_bitmap_start_block as usize;
        let data_bitmap_blocks = (left_blocks + BLOCK_NUM_BITS) / (BLOCK_NUM_BITS + 1);
        let data_bitmap = Bitmap::new(data_bitmap_start_block as usize, data_bitmap_blocks);
        let data_area_start_block = data_bitmap_start_block + data_bitmap_blocks as u32;

        let mut fs = Self {
            block_device: device.clone(),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block,
            data_area_start_block,
        };

        debug!("fs: {:?}, total: {}", fs, total_blocks);

        // clear all the blocks
        for i in 0..total_blocks {
            get_block_cache(i, device.clone())
                .lock()
                .modify(0, |data: &mut [u64; 64]| {
                    for x in data.iter_mut() {
                        *x = 0;
                    }
                    (true, ())
                });
        }

        // setup the super block
        get_block_cache(0, device.clone())
            .lock()
            .modify(0, |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks as u32,
                    inode_bitmap_blocks as u32,
                    inode_area_num_blocks as u32,
                    data_bitmap_blocks as u32,
                    (total_blocks - data_area_start_block as usize) as u32,
                );
                (true, ())
            });
        // write the '/' root directory
        assert_eq!(fs.alloc_inode(), Some(ROOT_INODE_ID as _));
        let (root_inode_block_id, root_inode_block_offset) = fs.inode_pos(ROOT_INODE_ID as _);
        get_block_cache(root_inode_block_id as _, device)
            .lock()
            .modify(root_inode_block_offset, |inode: &mut DiskInode| {
                inode.init(DiskInode::DISK_INODE_TYPE_DIR);
                (true, ())
            });

        // write back
        force_sync_all_cache();
        fs
    }

    pub fn device(&self) -> &Arc<dyn BlockDevice> {
        &self.block_device
    }

    pub fn root_inode(fs: &Arc<Mutex<Self>>) -> Inode {
        let f = fs.lock();
        let (block_id, block_offset) = f.inode_pos(0);
        Inode::new(block_id, block_offset, fs.clone(), f.device().clone())
    }

    /// returns the block id(without adding the inode area offset) and the actual offset in the block
    pub fn inode_pos(&self, inode_id: usize) -> (u32, usize) {
        let block_inner_id = inode_id / BLOCK_INODE_NUM;
        let offset = (inode_id % BLOCK_INODE_NUM) * core::mem::size_of::<DiskInode>();
        (self.inode_area_start_block + block_inner_id as u32, offset)
    }
}

impl EasyFileSystem {
    // inode id
    pub fn alloc_inode(&mut self) -> Option<usize> {
        self.inode_bitmap
            .alloc(&self.block_device)
    }

    // block id
    pub fn alloc_data_block(&mut self) -> Option<usize> {
        self.data_bitmap
            .alloc(&self.block_device)
            .map(|inner| inner + self.data_area_start_block as usize)
    }

    pub fn dealloc_inode(&mut self, inode_id: usize) {
        todo!()
    }

    // block id
    pub fn dealloc_data_block(&mut self, block_id: usize, clear: bool) {
        self.data_bitmap.dealloc(
            block_id - self.data_area_start_block as usize,
            self.block_device.clone(),
        );
        if !clear {
            return;
        }
        get_block_cache(
            block_id - self.data_area_start_block as usize,
            self.block_device.clone(),
        )
        .lock()
        .modify(0, |data: &mut [u64; 64]| {
            for x in data.iter_mut() {
                *x = 0;
            }
            (true, ())
        });
    }
}
