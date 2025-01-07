mod block_file;
pub use block_file::BlockFile;
pub use test_utils::*;

fn main() {
    println!("Hello, world!");
}

pub mod test_utils {
    use super::*;
    use easy_fs::{consts::BLOCK_NUM_BYTES, BlockDevice};
    use log::LevelFilter;
    use simple_logger::SimpleLogger;
    use std::sync::{Arc, Mutex};

    pub fn setup_logger(filter: LevelFilter) {
        let _ = SimpleLogger::new().with_level(filter).init().unwrap();
    }

    pub fn create_block_device() -> Arc<dyn BlockDevice> {
        let filename = "/tmp/test_block_file";
        // rm file if exists
        std::fs::remove_file(filename).ok();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();
        file.set_len(64 * BLOCK_NUM_BYTES as u64).unwrap();
        let block_file = BlockFile(Mutex::new(file));
        Arc::new(block_file)
    }

    pub fn create_fs_img() -> Arc<dyn BlockDevice> {
        let filename = "/tmp/easy_fs.img";
        // rm file if exists
        std::fs::remove_file(filename).ok();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();
        file.set_len(4096 * 2 * BLOCK_NUM_BYTES as u64).unwrap();
        let block_file = BlockFile(Mutex::new(file));
        Arc::new(block_file)
    }

    // too big to be used in tests
    pub fn create_big_fs_img() -> Arc<dyn BlockDevice> {
        let filename = "/tmp/easy_big_fs.img";
        // rm file if exists
        std::fs::remove_file(filename).ok();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();
        file.set_len(4096 * 4096 * 2 * BLOCK_NUM_BYTES as u64)
            .unwrap();
        let block_file = BlockFile(Mutex::new(file));
        Arc::new(block_file)
    }
}

///! cannot test all together lest the test might fail
#[cfg(test)]
mod basic_tests {

    use easy_fs::consts::BLOCK_NUM_BYTES;
    use easy_fs::{block_cache::force_sync_all_cache, consts::BLOCK_NUM_BITS, BitMapAlloc};

    use super::*;

    #[test]
    fn test_block_file() {
        let device = create_block_device();

        let mut buf = [0u8; BLOCK_NUM_BYTES];
        device.read_block(0, &mut buf);
        assert_eq!(&buf, &[0u8; BLOCK_NUM_BYTES]);
        buf[0] = 1;
        device.write_block(0, &buf);

        let mut new_buf = [0u8; BLOCK_NUM_BYTES];
        device.read_block(0, &mut new_buf);
        assert_eq!(new_buf[0], 1);
    }

    #[test]
    fn test_bitmap_simple_sync() {
        let device = create_block_device();
        let mut bitmap = easy_fs::Bitmap::new(1, 2);
        bitmap.clear(device.clone());
        assert_eq!(bitmap.alloc(&device), Some(0));
        assert_eq!(bitmap.alloc(&device), Some(1));
        assert_eq!(bitmap.alloc(&device), Some(2));

        // still in cache

        let mut buf = [0u8; BLOCK_NUM_BYTES];
        device.read_block(1, &mut buf);
        assert_eq!(buf[0], 0);

        force_sync_all_cache();

        // now is flushed to disk
        let mut buf = [0u8; BLOCK_NUM_BYTES];
        device.read_block(1, &mut buf);
        assert_eq!(buf[0], 7);
    }

    #[test]
    fn test_bitmap_alloc() {
        let device = create_block_device();
        let mut bitmap = easy_fs::Bitmap::new(1, 2);
        bitmap.clear(device.clone());

        for i in 0..64 {
            assert_eq!(bitmap.alloc(&device), Some(i));
        }
        for i in 0..32 {
            bitmap.dealloc(i, device.clone());
        }
        assert_eq!(bitmap.get_stat().free_inodes, BLOCK_NUM_BITS * 2 - 32);
        // reuse cycled bits
        for i in 0..32 {
            assert_eq!(bitmap.alloc(&device), Some(i));
        }

        assert_eq!(bitmap.get_stat().free_inodes, BLOCK_NUM_BITS * 2 - 64);

        let mut buf = [0u8; BLOCK_NUM_BYTES];
        device.read_block(1, &mut buf);
        assert_eq!(&buf[0..8], &[0; 8]);

        force_sync_all_cache();

        let mut buf = [0u8; BLOCK_NUM_BYTES];
        device.read_block(1, &mut buf);
        assert_eq!(&buf[0..8], &[u8::MAX; 8]);
    }
}

#[cfg(test)]
mod fs_tests {
    use easy_fs::{consts::BLOCK_NUM_BYTES, efs::EasyFileSystem, vfs::Inode};
    use spin::Mutex;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_ls() {
        setup_logger(log::LevelFilter::Debug);
        let device = create_fs_img();
        let fs = Arc::new(Mutex::new(EasyFileSystem::new(device, 4096 * 2, 2)));
        let root_inode = EasyFileSystem::root_inode(&fs);
        assert!(root_inode.is_dir());

        assert_eq!(root_inode.ls().unwrap().len(), 0);

        root_inode.create("dir1", true);
        root_inode.create("dir2", true);
        let ls = root_inode.ls().unwrap();
        assert_eq!(ls.len(), 2);

        root_inode.create("file1", false);
        root_inode.create("file2", false);
        let ls = root_inode.ls().unwrap();
        assert_eq!(ls.len(), 4);
        assert!(ls.contains(&"file1".to_owned()));
        assert!(ls.contains(&"file2".to_owned()));
        assert!(!ls.contains(&"file3".to_owned()));

        let inode = root_inode.find("file1").unwrap();
        assert!(!inode.is_dir());
    }

    fn ident_print(inode: &Inode, depth: usize) {
        let children = inode.ls().unwrap();
        for child in children {
            let prefix = "  ".repeat(depth);
            println!("{}{}", prefix, child);
            let child_inode = inode.find(&child).unwrap();
            if child_inode.is_dir() {
                ident_print(&child_inode, depth + 1);
            }
        }
    }

    #[test]
    fn test_tree() {
        setup_logger(log::LevelFilter::Debug);
        let device = create_fs_img();
        let fs = Arc::new(Mutex::new(EasyFileSystem::new(device, 4096 * 2, 2)));
        let root_inode = EasyFileSystem::root_inode(&fs);
        println!("root inode: {:?}", root_inode);

        root_inode.create("dir1", true);
        root_inode.create("dir2", true);
        root_inode.create("file1", false);
        root_inode.create("file2", false);
        root_inode.create("file3", false);

        println!("{:?}", root_inode.ls().unwrap());

        let dir1 = root_inode.find("dir1").unwrap();
        println!("dir1 inode: {:?}", dir1);
        dir1.create("dir1_file1", false);
        dir1.create("dir1_file2", false);

        println!("{:?}", root_inode.ls().unwrap());

        let dir2 = root_inode.find("dir2").unwrap();
        dir2.create("dir2_file1", false);
        dir2.create("dir2_file2", false);

        ident_print(&root_inode, 0);
    }

    #[test]
    fn test_absolute_path() {
        setup_logger(log::LevelFilter::Debug);
        let device = create_fs_img();
        let fs = Arc::new(Mutex::new(EasyFileSystem::new(device, 4096 * 2, 2)));
        let root_inode = EasyFileSystem::root_inode(&fs);

        root_inode.create("dir1", true);
        let dir1 = root_inode.find_absolute("/dir1").unwrap();
        dir1.create("dir2", true);
        let dir2 = root_inode.find_absolute("/dir1/dir2").unwrap();
        dir2.create("dir3", true);
        let dir3 = root_inode.find_absolute("/dir1/dir2/dir3").unwrap();
        dir3.create("file1", false);

        let file = root_inode.find_absolute("/dir1/dir2/dir3/file1").unwrap();
        assert!(!file.is_dir());

        root_inode.create("dir_a", true);
        let dir_a = root_inode.find_absolute("/dir_a").unwrap();
        dir_a.create("file_a", false);
        assert!(root_inode.find_absolute("/dir_a/file_a").unwrap().is_file());
        assert!(root_inode.find_absolute("/dir_a/file_a/dirx").is_none());

        root_inode.resize(0);
    }

    #[test]
    fn test_large_rw1() {
        setup_logger(log::LevelFilter::Debug);
        let device = create_fs_img();
        let fs = Arc::new(Mutex::new(EasyFileSystem::new(device, 4096 * 2, 2)));
        let root_inode = EasyFileSystem::root_inode(&fs);

        root_inode.create("file", false);
        let file_inode = root_inode.find("file").unwrap();

        const SIZE: usize = 28 + 128 + 6;

        let buf = [1u8; BLOCK_NUM_BYTES * SIZE];
        file_inode.write_at(0, &buf[..]);

        assert_eq!(file_inode.size(), BLOCK_NUM_BYTES * SIZE);

        let mut read_buf = [0u8; BLOCK_NUM_BYTES * SIZE];
        file_inode.read_at(0, &mut read_buf[..]);
        assert_eq!(&buf[..], &read_buf[..]);

        const SIZE2: usize = 28 + 128 + 1 * 128 + 1;

        let buf2 = [2u8; BLOCK_NUM_BYTES * (SIZE2 - SIZE)];
        file_inode.write_at(BLOCK_NUM_BYTES * SIZE, &buf2[..]);

        let mut read_buf2 = [0u8; BLOCK_NUM_BYTES * (SIZE2 - SIZE)];
        file_inode.read_at(BLOCK_NUM_BYTES * SIZE, &mut read_buf2[..]);
        assert_eq!(&buf2[..], &read_buf2[..]);

        assert_eq!(file_inode.size(), BLOCK_NUM_BYTES * SIZE2);

        file_inode.resize(BLOCK_NUM_BYTES * SIZE);
        assert_eq!(file_inode.size(), BLOCK_NUM_BYTES * SIZE);
        file_inode.resize(0);
        assert_eq!(file_inode.size(), 0);
    }
}
