use std::{
    io::{Read, Seek, Write},
    sync::Mutex,
};

use easy_fs::{consts::BLOCK_NUM_BYTES, BlockDevice};

pub struct BlockFile(pub(crate) Mutex<std::fs::File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        let offset = block_id * BLOCK_NUM_BYTES;
        file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
        assert_eq!(
            file.read(buf).unwrap(),
            BLOCK_NUM_BYTES,
            "not a complete block"
        );
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        let offset = block_id * BLOCK_NUM_BYTES;
        file.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
        assert_eq!(
            file.write(buf).unwrap(),
            BLOCK_NUM_BYTES,
            "not a complete block"
        );
        file.flush().unwrap();
    }
}