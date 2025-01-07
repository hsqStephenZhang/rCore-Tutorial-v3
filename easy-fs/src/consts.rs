pub const EFS_MAGIC: u32 = 0x5f5f_4553;
pub const BLOCK_NUM_BYTES: usize = 512;
pub const BLOCK_NUM_BITS: usize = BLOCK_NUM_BYTES * 8;
pub const NUM_INDIRECT_BLOCK_ENTRIES: usize = BLOCK_NUM_BYTES / 4;
pub const INODE_DIRECT_COUNT: usize = 28;
// every entry is u32, 4 bytes, so the total entry that can be stored in a block is BLOCK_NUM_BYTES / 4 = 128
pub const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
pub const INDIRECT1_BOUND: usize = DIRECT_BOUND + NUM_INDIRECT_BLOCK_ENTRIES;
pub const INDIRECT2_BOUND: usize =
    INDIRECT1_BOUND + NUM_INDIRECT_BLOCK_ENTRIES * NUM_INDIRECT_BLOCK_ENTRIES;

pub const DIRECT_BOUND_BYTES: usize = DIRECT_BOUND * BLOCK_NUM_BYTES;
pub const INDIRECT1_BOUND_BYTES: usize = INDIRECT1_BOUND * BLOCK_NUM_BYTES;
pub const INDIRECT2_BOUND_BYTES: usize = INDIRECT2_BOUND * BLOCK_NUM_BYTES;

// the 28th bit shall be set to '/0'
pub const FILE_NAME_LIMIT: usize = 27;

pub const ROOT_INODE_ID: u32 = 0;