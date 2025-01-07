use core::fmt;

use alloc::{borrow::ToOwned, string::String, sync::Arc, vec::Vec};
use log::debug;
use spin::{Mutex, MutexGuard};

use crate::{
    block_cache::get_block_cache,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode},
    BlockDevice, FILE_NAME_LIMIT,
};

pub struct Inode {
    pub block_id: usize,
    pub block_offset: usize,
    pub fs: Arc<Mutex<EasyFileSystem>>,
    // avoid access of mutex of fs every time
    pub block_device: Arc<dyn BlockDevice>,
}

impl fmt::Debug for Inode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inode")
            .field("block_id", &self.block_id)
            .field("block_offset", &self.block_offset)
            .finish()
    }
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> (bool, V)) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
}

impl Inode {
    pub fn is_dir(&self) -> bool {
        self.read_disk_inode(|inode| inode.is_dir())
    }
    pub fn is_file(&self) -> bool {
        self.read_disk_inode(|inode| inode.is_file())
    }
}

impl Inode {
    // TODO: optimize the return type, add result to indicate the error
    pub fn create(&self, name: &str, is_dir: bool) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self.read_disk_inode(|inode: &DiskInode| !inode.is_dir()) {
            return None;
        }
        assert!(name.len() <= FILE_NAME_LIMIT);
        let old_size = self.read_disk_inode(|inode| inode.size as usize);
        assert!(old_size % core::mem::size_of::<DirEntry>() == 0);
        self.increase_size_locked(old_size + core::mem::size_of::<DirEntry>(), &mut fs);

        let inode = fs.alloc_inode()?;
        let entry = DirEntry::new(name, inode as _);
        let num_write = self.modify_disk_inode(|inode| {
            let num_write =
                inode.write_at_locked(old_size, entry.as_bytes(), self.block_device.clone());
            (true, num_write)
        });
        assert_eq!(num_write, core::mem::size_of::<DirEntry>());

        let (block_id, block_offset) = fs.inode_pos(inode);
        let new_inode = Inode::new(
            block_id,
            block_offset,
            self.fs.clone(),
            fs.block_device.clone(),
        );
        new_inode.modify_disk_inode(|inode| {
            inode.init(if is_dir {
                DiskInode::DISK_INODE_TYPE_DIR
            } else {
                DiskInode::DISK_INODE_TYPE_FILE
            });
            (true, ())
        });

        Some(Arc::new(new_inode))
    }

    pub fn find_absolute(&self, path: &str) -> Option<Arc<Inode>> {
        let mut root = Arc::new(EasyFileSystem::root_inode(&self.fs));
        for name in path.split("/") {
            if name.is_empty() {
                continue;
            }
            root = root.find(name)?;
        }
        Some(root)
    }

    pub fn find_relative(&self, path: &str, cwd: &str) -> Option<Arc<Inode>> {
        // resolve the absolute path and then call find_absolute
        todo!()
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();

        self.read_disk_inode(|disk_inode| {
            disk_inode
                .find_block_id_by_name_locked(name, fs.device().clone())
                .map(|inode_id| {
                    let (block_id, block_offset) = fs.inode_pos(inode_id as _);
                    Arc::new(Self::new(
                        block_id,
                        block_offset,
                        self.fs.clone(),
                        self.block_device.clone(),
                    ))
                })
        })
    }

    pub fn ls(&self) -> Option<Vec<String>> {
        let _fs = self.fs.lock();
        let res = self.read_disk_inode(|inode| {
            if inode.is_dir() {
                let size = inode.size as usize;
                assert!(size % core::mem::size_of::<DirEntry>() == 0);
                let num_entries = size / core::mem::size_of::<DirEntry>();
                debug!(
                    "num_entries of inode[{}-{}]: {}",
                    self.block_id, self.block_offset, num_entries
                );
                let mut entries = Vec::with_capacity(num_entries);
                for i in 0..num_entries {
                    let mut entry = DirEntry::default();
                    assert_eq!(
                        inode.read_at_locked(
                            i * core::mem::size_of::<DirEntry>(),
                            entry.as_bytes_mut(),
                            self.block_device.clone()
                        ),
                        core::mem::size_of::<DirEntry>()
                    );
                    entries.push(entry.name().to_owned());
                }
                Some(entries)
            } else {
                None
            }
        });
        res
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|inode| inode.read_at_locked(offset, buf, self.block_device.clone()))
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        log::debug!("increase size");
        self.increase_size_locked(offset + buf.len(), &mut fs);
        log::debug!("increase size finished");
        self.modify_disk_inode(|inode| {
            let num_write = inode.write_at_locked(offset, buf, self.block_device.clone());
            (true, num_write)
        })
    }

    pub fn size(&self) -> usize {
        self.read_disk_inode(|inode| inode.size as usize)
    }

    // default will not destroy the data blocks
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.decrease_size_locked(0, &mut fs, false);
    }

    // default will not destroy the data blocks
    pub fn resize(&self, new_size: usize) {
        let mut fs = self.fs.lock();
        if new_size < self.size() {
            self.decrease_size_locked(new_size, &mut fs, false);
        } else {
            self.increase_size_locked(new_size, &mut fs);
        }
    }

    fn decrease_size_locked(
        &self,
        new_size: usize,
        fs: &mut MutexGuard<'_, EasyFileSystem>,
        destroy: bool,
    ) {
        if self.read_disk_inode(|inode| (inode.size as usize) <= new_size) {
            return;
        }
        let num_blocks_to_free = self.read_disk_inode(|inode| inode.num_blocked_to_free(new_size));
        self.modify_disk_inode(|inode| {
            let to_free = inode.decrease_size_locked(new_size, fs.device().clone());
            assert_eq!(to_free.len(), num_blocks_to_free);
            if destroy {
                for data_block_id in to_free {
                    fs.dealloc_data_block(data_block_id as usize, true);
                }
            }
            (true, ())
        });
    }

    // should hold the lock of fs and pass via argument
    fn increase_size_locked(
        &self,
        new_size: usize,
        fs: &mut MutexGuard<'_, EasyFileSystem>,
    ) -> bool {
        if self.read_disk_inode(|inode| (inode.size as usize) >= new_size) {
            return false;
        }

        // might be zero
        let num_blocks_needed = self.read_disk_inode(|inode| inode.num_blocks_needed(new_size));
        let mut allocated_blocks = Vec::new();
        for i in 0..num_blocks_needed {
            let block_id = match fs.alloc_data_block() {
                Some(block_id) => block_id,
                None => {
                    panic!(
                        "no enough blocks, panic at {}, need {} blocks",
                        i, num_blocks_needed
                    );
                }
            };
            allocated_blocks.push(block_id as u32);
        }
        self.modify_disk_inode(|inode| {
            inode.increase_size_locked(new_size, allocated_blocks, fs.device().clone());
            (true, ())
        });
        true
    }
}
