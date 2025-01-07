use core::fmt;

use alloc::{sync::Arc, vec::Vec};

use crate::{
    block_cache::get_block_cache, BlockDevice, BLOCK_NUM_BYTES, DIRECT_BOUND, EFS_MAGIC,
    FILE_NAME_LIMIT, INDIRECT1_BOUND, INDIRECT2_BOUND, INODE_DIRECT_COUNT,
    NUM_INDIRECT_BLOCK_ENTRIES,
};

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }

    // TODO: add more specific validation checks
    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

type DiskInodeType = u32;

#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}

pub const BLOCK_INODE_NUM: usize = BLOCK_NUM_BYTES / core::mem::size_of::<DiskInode>();

const_assert_eq!(core::mem::size_of::<DiskInode>(), 128);

impl DiskInode {
    pub const DISK_INODE_TYPE_FILE: DiskInodeType = 0;
    pub const DISK_INODE_TYPE_DIR: DiskInodeType = 1;

    pub fn is_dir(&self) -> bool {
        self.type_ == Self::DISK_INODE_TYPE_DIR
    }

    pub fn is_file(&self) -> bool {
        self.type_ == Self::DISK_INODE_TYPE_FILE
    }

    pub fn init(&mut self, type_: DiskInodeType) {
        self.type_ = type_;
    }
}

impl DiskInode {
    pub fn new(type_: DiskInodeType) -> Self {
        Self {
            size: 0,
            direct: [0; INODE_DIRECT_COUNT],
            indirect1: 0,
            indirect2: 0,
            type_,
        }
    }
}

// [u32; 128]
type IndirectBlock = [u32; BLOCK_NUM_BYTES / 4];

impl DiskInode {
    // inner_id should be offset / BLOCK_NUM_BYTES
    pub fn get_block_id(&self, inode_id: usize, device: Arc<dyn BlockDevice>) -> u32 {
        let mut inner_id = inode_id;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, device.clone())
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else if inner_id < INDIRECT2_BOUND {
            inner_id -= INDIRECT1_BOUND;
            let indirect2_block_id = get_block_cache(self.indirect2 as usize, device.clone())
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id / NUM_INDIRECT_BLOCK_ENTRIES]
                });
            get_block_cache(indirect2_block_id as _, device)
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id % NUM_INDIRECT_BLOCK_ENTRIES]
                })
        } else {
            panic!(
                "out of bound, inner_id: {}, max supported data block count: {}",
                inner_id, INDIRECT2_BOUND
            );
        }
    }

    // should hold the lock of fs
    pub fn find_block_id_by_name_locked(
        &self,
        name: &str,
        device: Arc<dyn BlockDevice>,
    ) -> Option<u32> {
        let file_count = (self.size as usize) / core::mem::size_of::<DirEntry>();
        let mut entry = DirEntry::default();
        for i in 0..file_count {
            assert_eq!(
                self.read_at_locked(DIRENT_SZ * i, entry.as_bytes_mut(), device.clone()),
                DIRENT_SZ,
            );
            if entry.name() == name {
                return Some(entry.inode() as u32);
            }
        }
        None
    }

    fn data_blocks(size: usize) -> usize {
        (size + BLOCK_NUM_BYTES - 1) / BLOCK_NUM_BYTES
    }

    fn total_blocks(size: usize) -> usize {
        let data_blocks = Self::data_blocks(size);
        let mut total = data_blocks;
        if data_blocks > DIRECT_BOUND {
            total += 1;
        }
        if data_blocks > INDIRECT1_BOUND {
            total += 1;

            let indirect2_blocks = (total - INDIRECT1_BOUND + NUM_INDIRECT_BLOCK_ENTRIES - 1)
                / NUM_INDIRECT_BLOCK_ENTRIES;
            total += indirect2_blocks;
        }
        total
    }

    pub fn num_blocks_needed(&self, new_size: usize) -> usize {
        assert!(new_size >= self.size as usize);
        Self::total_blocks(new_size) - Self::total_blocks(self.size as usize)
    }

    pub fn num_blocked_to_free(&self, new_size: usize) -> usize {
        assert!(new_size <= self.size as usize);
        Self::total_blocks(self.size as usize) - Self::total_blocks(new_size)
    }

    // should hold the lock of fs
    pub fn increase_size_locked(
        &mut self,
        new_size: usize,
        data_block_ids: Vec<u32>,
        device: Arc<dyn BlockDevice>,
    ) {
        assert_eq!(self.num_blocks_needed(new_size), data_block_ids.len());
        // log::info!("data_block_ids len: {}", data_block_ids.len());
        let old_blocks = Self::data_blocks(self.size as usize);
        self.size = new_size as u32;
        let new_blocks = Self::data_blocks(new_size);
        let mut iter = data_block_ids.into_iter();

        let mut block = old_blocks;

        if block < DIRECT_BOUND {
            let start = block;
            let end = new_blocks.min(DIRECT_BOUND);
            for i in start..end {
                self.direct[i] = iter.next().unwrap();
            }
            block = end;
        }
        if block >= new_blocks {
            assert!(iter.next().is_none());
            return;
        }
        if block < INDIRECT1_BOUND {
            if self.indirect1 == 0 {
                self.indirect1 = iter.next().unwrap();
            }

            let start = block;
            let end = new_blocks.min(INDIRECT1_BOUND);
            get_block_cache(self.indirect1 as usize, device.clone())
                .lock()
                .modify(0, |indirect_block: &mut IndirectBlock| {
                    for i in start..end {
                        indirect_block[i - DIRECT_BOUND] = iter.next().unwrap();
                    }
                    (true, ())
                });
            block = end;
        }
        if block >= new_blocks {
            assert!(iter.next().is_none());
            return;
        }
        if block < INDIRECT2_BOUND {
            if self.indirect2 == 0 {
                self.indirect2 = iter.next().unwrap();
            }
            let mut start = block - INDIRECT1_BOUND;
            let end = new_blocks.min(INDIRECT2_BOUND) - INDIRECT1_BOUND;
            log::info!(
                "increase, indirect2, inner start: {} * 128 + {}",
                start / NUM_INDIRECT_BLOCK_ENTRIES,
                start % NUM_INDIRECT_BLOCK_ENTRIES
            );
            log::info!(
                "increase, indirect2, inner end: {} * 128 + {}",
                end / NUM_INDIRECT_BLOCK_ENTRIES,
                end % NUM_INDIRECT_BLOCK_ENTRIES
            );
            get_block_cache(self.indirect2 as _, device.clone())
                .lock()
                .modify(0, |indirect_block: &mut IndirectBlock| {
                    while start < end {
                        let block_start_idx = start / NUM_INDIRECT_BLOCK_ENTRIES;
                        let block_start = block_start_idx * NUM_INDIRECT_BLOCK_ENTRIES;
                        let block_end = (block_start_idx + 1) * NUM_INDIRECT_BLOCK_ENTRIES;

                        let real_start = start.max(block_start);
                        let real_end = end.min(block_end);

                        if real_start == block_start {
                            indirect_block[block_start_idx] = iter.next().unwrap();
                        }
                        get_block_cache(indirect_block[block_start_idx] as _, device.clone())
                            .lock()
                            .modify(0, |indirect_block: &mut IndirectBlock| {
                                let indirect1_start = real_start % NUM_INDIRECT_BLOCK_ENTRIES;
                                let num = real_end - real_start;
                                for i in (indirect1_start..).take(num) {
                                    indirect_block[i] = iter.next().unwrap();
                                }
                                (true, ())
                            });
                        start = real_end;
                    }
                    (true, ())
                });
        }
        assert!(iter.next().is_none());
    }

    pub fn decrease_size_locked(
        &mut self,
        new_size: usize,
        device: Arc<dyn BlockDevice>,
    ) -> Vec<u32> {
        let mut to_free = Vec::new();
        let num_to_free = self.num_blocked_to_free(new_size);
        let old_blocks = Self::data_blocks(self.size as usize);
        self.size = new_size as u32;
        // new blocks is smaller
        let new_blocks = Self::data_blocks(new_size);

        let mut block = new_blocks;

        if block < DIRECT_BOUND {
            let start = block;
            let end = old_blocks.min(DIRECT_BOUND);
            for i in start..end {
                to_free.push(self.direct[i]);
                self.direct[i] = 0;
            }
            block = end;
        }
        if block >= old_blocks {
            assert!(to_free.len() == num_to_free);
            return to_free;
        }
        if block < INDIRECT1_BOUND {
            let indirect1 = if block == DIRECT_BOUND {
                let indirect1 = self.indirect1;
                self.indirect1 = 0;
                to_free.push(indirect1);
                indirect1
            } else {
                self.indirect1
            };

            let start = block;
            let end = old_blocks.min(INDIRECT1_BOUND);
            get_block_cache(indirect1 as usize, device.clone())
                .lock()
                .modify(0, |indirect_block: &mut IndirectBlock| {
                    for i in start..end {
                        to_free.push(indirect_block[i - DIRECT_BOUND]);
                        indirect_block[i - DIRECT_BOUND] = 0;
                    }
                    (true, ())
                });
            block = end;
        }
        if block >= old_blocks {
            assert!(to_free.len() == num_to_free);
            return to_free;
        }
        if block < INDIRECT2_BOUND {
            let indirect2 = if block == INDIRECT1_BOUND {
                let indirect2 = self.indirect2;
                self.indirect2 = 0;
                to_free.push(indirect2);
                indirect2
            } else {
                self.indirect2
            };

            let mut start = block - INDIRECT1_BOUND;
            let end = old_blocks.min(INDIRECT2_BOUND) - INDIRECT1_BOUND;
            log::info!(
                "decrease indirect2, inner start: {} * 128 + {}",
                start / NUM_INDIRECT_BLOCK_ENTRIES,
                start % NUM_INDIRECT_BLOCK_ENTRIES
            );
            log::info!(
                "decrease indirect2, inner end: {} * 128 + {}",
                end / NUM_INDIRECT_BLOCK_ENTRIES,
                end % NUM_INDIRECT_BLOCK_ENTRIES
            );
            get_block_cache(indirect2 as _, device.clone())
                .lock()
                .modify(0, |indirect_block: &mut IndirectBlock| {
                    while start < end {
                        let block_start_idx = start / NUM_INDIRECT_BLOCK_ENTRIES;
                        let block_start = block_start_idx * NUM_INDIRECT_BLOCK_ENTRIES;
                        let block_end = (block_start_idx + 1) * NUM_INDIRECT_BLOCK_ENTRIES;

                        let real_start = start.max(block_start);
                        let real_end = end.min(block_end);

                        if real_start == block_start {
                            to_free.push(indirect_block[block_start_idx]);
                            indirect_block[block_start_idx] = 0;
                        }
                        get_block_cache(indirect_block[block_start_idx] as _, device.clone())
                            .lock()
                            .modify(0, |indirect_block: &mut IndirectBlock| {
                                let indirect1_start = real_start % NUM_INDIRECT_BLOCK_ENTRIES;
                                let num = real_end - real_start;
                                for i in (indirect1_start..).take(num) {
                                    to_free.push(indirect_block[i]);
                                    indirect_block[i] = 0;
                                }
                                (true, ())
                            });
                        start = real_end;
                    }
                    (true, ())
                });
        }

        assert_eq!(to_free.len(), num_to_free);
        return to_free;
    }

    // should hold the lock of fs
    pub fn read_at_locked(
        &self,
        offset: usize,
        buf: &mut [u8],
        device: Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (self.size as usize).min(offset + buf.len());
        if start >= end {
            return 0;
        }
        let mut start_block = start / BLOCK_NUM_BYTES;
        let mut read_size = 0;
        while start < end {
            let current_block_end_addr = end.min((start_block + 1) * BLOCK_NUM_BYTES);
            // num of bytes to read in current block
            let current_block_num_bytes = current_block_end_addr - start;
            let dst = &mut buf[read_size..read_size + current_block_num_bytes];

            let block_id = self.get_block_id(start_block, device.clone());
            get_block_cache(block_id as _, device.clone()).lock().read(
                0,
                |data_block: &DataBlock| {
                    let current_block_start_offset = start % BLOCK_NUM_BYTES;
                    let data_block = &data_block[current_block_start_offset
                        ..current_block_start_offset + current_block_num_bytes];
                    assert_eq!(data_block.len(), dst.len());
                    dst.copy_from_slice(data_block);
                    read_size += current_block_num_bytes;
                },
            );
            start += current_block_num_bytes;
            start_block += 1;
        }
        read_size
    }

    // should hold the lock of fs
    pub fn write_at_locked(
        &self,
        offset: usize,
        buf: &[u8],
        device: Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (self.size as usize).min(offset + buf.len());
        if start >= end {
            return 0;
        }
        let mut start_block = start / BLOCK_NUM_BYTES;
        let mut write_size = 0;
        while start < end {
            let current_block_end_addr = end.min((start_block + 1) * BLOCK_NUM_BYTES);
            // num of bytes to read in current block
            let current_block_num_bytes = current_block_end_addr - start;
            let src = &buf[write_size..write_size + current_block_num_bytes];

            let block_id = self.get_block_id(start_block, device.clone());
            get_block_cache(block_id as _, device.clone())
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    let current_block_start_offset = start % BLOCK_NUM_BYTES;
                    let data_block = &mut data_block[current_block_start_offset
                        ..current_block_start_offset + current_block_num_bytes];
                    assert_eq!(data_block.len(), src.len());
                    data_block.copy_from_slice(src);
                    write_size += current_block_num_bytes;
                    (true, ())
                });
            start += current_block_num_bytes;
            start_block += 1;
        }
        write_size
    }
}

// [u8; 512]
type DataBlock = [u8; BLOCK_NUM_BYTES];
pub const DIRENT_SZ: usize = core::mem::size_of::<DirEntry>();
pub const NUM_ENTRIES_PER_BLOCK: usize = BLOCK_NUM_BYTES / DIRENT_SZ;

const_assert!(DIRENT_SZ == 32);
const_assert!(NUM_ENTRIES_PER_BLOCK == 16);

#[repr(C)]
pub struct DirEntry {
    pub name: [u8; FILE_NAME_LIMIT + 1],
    pub inode: u32,
}

impl Default for DirEntry {
    fn default() -> Self {
        Self {
            name: [0; FILE_NAME_LIMIT + 1],
            inode: 0,
        }
    }
}

impl DirEntry {
    pub fn new(name: &str, inode: u32) -> Self {
        let mut name1 = [0_u8; FILE_NAME_LIMIT + 1];
        let name_bytes = name.as_bytes();
        assert!(name_bytes.len() <= FILE_NAME_LIMIT);
        name1[0..name_bytes.len()].copy_from_slice(name_bytes);
        Self { name: name1, inode }
    }

    pub fn name(&self) -> &str {
        let len = self.name.iter().position(|&x| x == 0).unwrap();
        core::str::from_utf8(&self.name[0..len]).unwrap()
    }

    pub fn inode(&self) -> u32 {
        self.inode
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const _,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self as *mut Self as *mut _,
                core::mem::size_of::<Self>(),
            )
        }
    }
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "DirEntry {{ name: {:?}, inode: {} }}",
            self.name(),
            self.inode
        )
    }
}
