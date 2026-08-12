extern crate alloc;
use crate::error::Error::Failed;
use crate::error::Result;
use crate::fs::minix::*;
use crate::mutex::Mutex;
use alloc::vec;
use alloc::vec::Vec;

static MINIX_FS: Mutex<Option<MinixFs>> = Mutex::new(None);

pub struct MinixFs {
    image: Vec<u8>,
    block_size: usize,
}
impl MinixFs {
    pub fn init() -> Result<()> {
        let mut global_fs = MINIX_FS.lock();

        const BLOCK_SIZE: usize = 1024;
        let size = 512 * 32;
        let mut mem = vec![0u8; size];

        init_minixfs(&mut mem, BLOCK_SIZE);

        *global_fs = Some(MinixFs {
            image: mem,
            block_size: BLOCK_SIZE,
        });

        Ok(())
    }

    pub fn show_directory_tree() -> Result<()> {
        let mut global_fs = MINIX_FS.lock();

        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let root_inode = minix3_inode::new(0);

        root_inode.show_directory_tree(&mut fs.image, fs.block_size)
    }

    pub fn read(file_path: &[u8]) -> Result<Vec<u8>> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let inode = minix3_inode::new(0);
        let data = inode.read(&mut fs.image, file_path, fs.block_size)?;
        // lock解除後も使えるようにvecを用いてコピーした値を返す
        Ok(data.to_vec())
    }

    pub fn write(file_path: &[u8], data: &[u8]) -> Result<()> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let inode = minix3_inode::new(0);

        inode.write(&mut fs.image, file_path, fs.block_size, data)
    }

    pub fn delete(file_path: &[u8]) -> Result<()> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let inode = minix3_inode::new(0);

        inode.delete_dir_entry(&mut fs.image, file_path, fs.block_size)
    }

    pub fn lookup_iter(file_path: &[u8]) -> Result<u32> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let inode = minix3_inode::new(0);
        inode.lookup_iter(file_path, &mut fs.image, fs.block_size)
    }

    pub fn mkdir(file_path: &[u8]) -> Result<u32> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let inode = minix3_inode::new(0);
        inode.mkdir(&mut fs.image, file_path, fs.block_size)
    }

    pub fn create_file(file_path: &[u8]) -> Result<u32> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let inode = minix3_inode::new(0);
        inode.create_file(&mut fs.image, fs.block_size, file_path)
    }
}
