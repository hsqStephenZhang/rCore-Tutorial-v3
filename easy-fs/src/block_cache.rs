use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use spin::Mutex;

use crate::{block_dev::BlockDevice, BLOCK_NUM_BYTES};

lazy_static::lazy_static! {
    pub static ref MANAGER: Arc<spin::Mutex<dyn BlockCacheManager>> = Arc::new(spin::Mutex::new(BlockCacheManagerImpl::new()));
}

pub fn get_block_cache(block_id: usize, device: Arc<dyn BlockDevice>) -> Arc<Mutex<BlockCache>> {
    MANAGER.lock().get_block_cache(block_id, device)
}

pub fn force_sync_cache(block_id: usize, device: Arc<dyn BlockDevice>) {
    get_block_cache(block_id, device).lock().sync();
}

pub fn force_sync_all_cache() {
    MANAGER.lock().sync_all();
}

// pub fn debug_cache() {
//     let manager = MANAGER.lock();
// }

pub struct BlockCache {
    // 4096 bits
    data: [u8; BLOCK_NUM_BYTES],
    block_id: usize,
    // used to write back to device
    device: Arc<dyn BlockDevice>,
    modified: bool,
}

impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut data = [0; BLOCK_NUM_BYTES];
        block_device.read_block(block_id, &mut data);
        Self {
            data,
            block_id,
            device: block_device,
            modified: false,
        }
    }

    #[inline]
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.data[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_NUM_BYTES);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }

    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_NUM_BYTES);
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> (bool, V)) -> V {
        let (modified, val) = f(self.get_mut(offset));
        self.modified = modified;
        val
    }
}

impl BlockCache {
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.device.write_block(self.block_id, &self.data);
        }
    }
}

impl Drop for BlockCache {
    // write back to device
    fn drop(&mut self) {
        if self.modified {
            self.sync();
        }
    }
}

pub trait BlockCacheManager: Send + Sync {
    fn get_block_cache(
        &mut self,
        block_id: usize,
        device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>>;

    fn sync_all(&mut self);
}

pub struct BlockCacheManagerImpl {
    // (block_id, block_cache)
    cache: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManagerImpl {
    pub const BLOCK_CACHE_SIZE: usize = 16;

    pub fn new() -> Self {
        BlockCacheManagerImpl {
            cache: VecDeque::with_capacity(Self::BLOCK_CACHE_SIZE),
        }
    }
}

impl BlockCacheManager for BlockCacheManagerImpl {
    fn get_block_cache(
        &mut self,
        block_id: usize,
        device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, cache)) = self.cache.iter().find(|(id, _)| *id == block_id) {
            return cache.clone();
        } else {
            if self.cache.len() == Self::BLOCK_CACHE_SIZE {
                if let Some((idx, _)) = self
                    .cache
                    .iter()
                    .enumerate()
                    .find(|(_, (_, cache_item))| Arc::strong_count(cache_item) == 1)
                {
                    self.cache.drain(idx..idx + 1);
                } else {
                    panic!("block cache is full");
                }
            }

            let cache = Arc::new(Mutex::new(BlockCache::new(block_id, device)));
            self.cache.push_back((block_id, cache.clone()));
            cache
        }
    }
    
    fn sync_all(&mut self) {
        for (_, cache) in self.cache.iter() {
            cache.lock().sync();
        }
    }
}
