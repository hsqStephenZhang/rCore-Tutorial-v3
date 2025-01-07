use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_NUM_BITS};

///! Bitmap with block id range [start_block_id, start_block_id + num_blocks)
#[derive(Debug, Clone, Copy)]
pub struct Bitmap {
    start_block_id: usize,
    num_blocks: usize,
    allocated: usize,
}

pub struct BitmapStat {
    pub max_inodes: usize,
    pub free_inodes: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, num_blocks: usize) -> Self {
        Self {
            start_block_id,
            num_blocks,
            allocated: 0,
        }
    }
}

impl Bitmap {
    /// read
    pub fn max_inodes(&self) -> usize {
        self.num_blocks * BLOCK_NUM_BITS
    }

    pub fn free_inodes(&self) -> usize {
        self.max_inodes() - self.allocated
    }

    pub fn get_stat(&self) -> BitmapStat {
        BitmapStat {
            max_inodes: self.max_inodes(),
            free_inodes: self.free_inodes(),
        }
    }

    /// write

    pub fn clear(&mut self, block_device: Arc<dyn BlockDevice>) {
        for i in 0..self.num_blocks {
            get_block_cache(self.start_block_id + i, block_device.clone())
                .lock()
                .modify(0, |bitmap_block: &mut BitMapBlock| {
                    for x in bitmap_block.iter_mut() {
                        *x = 0;
                    }
                    (true, ())
                });
        }
    }
}

// 4096 bits as the bitmap block
type BitMapBlock = [u64; 64];

pub trait BitMapAlloc {
    fn alloc(&mut self, block_device: &Arc<dyn BlockDevice>) -> Option<usize>;
    fn dealloc(&mut self, block_id: usize, block_device: Arc<dyn BlockDevice>);
}

impl BitMapAlloc for Bitmap {
    // first match alloc
    fn alloc(&mut self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for i in 0..self.num_blocks {
            let bit_idx = get_block_cache(self.start_block_id + i, block_device.clone())
                .lock()
                .modify(0, |bitmap_block: &mut BitMapBlock| {
                    if let Some((bit_block_idx, bit_block_val)) = bitmap_block
                        .iter_mut()
                        .enumerate()
                        .find(|(_, x)| **x != u64::MAX)
                    {
                        let bit_idx_of_bitmap_block = bit_block_val.trailing_ones() as usize;
                        *bit_block_val |= 1 << bit_idx_of_bitmap_block;
                        // should mark as modified
                        return (true, Some(bit_block_idx * 64 + bit_idx_of_bitmap_block));
                    } else {
                        return (false, None);
                    }
                });
            if let Some(bit_idx) = bit_idx {
                self.allocated += 1;
                return Some(i * BLOCK_NUM_BITS + bit_idx);
            }
        }
        None
    }

    // direct return
    fn dealloc(&mut self, block_id: usize, block_device: Arc<dyn BlockDevice>) {
        let i = block_id / BLOCK_NUM_BITS;
        let bit_idx = block_id % BLOCK_NUM_BITS;
        get_block_cache(self.start_block_id + i, block_device)
            .lock()
            .modify(0, |bitmap_block: &mut BitMapBlock| {
                let bit_block_idx = bit_idx / 64;
                let bit_idx_of_bitmap_block = bit_idx % 64;
                assert!(bitmap_block[bit_block_idx] & (1 << bit_idx_of_bitmap_block) != 0);
                bitmap_block[bit_block_idx] &= !(1 << bit_idx_of_bitmap_block);
                self.allocated -= 1;
                (true, ())
            });
    }
}
