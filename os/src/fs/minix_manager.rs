extern crate alloc;
use crate::error::Error::Failed;
use crate::error::Result;
use crate::fs::minix::*;
use crate::mutex::Mutex;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;

static MINIX_FS: Mutex<Option<MinixFs>> = Mutex::new(None);

pub struct MinixFs {
    image: Vec<u8>,
    block_size: usize,
}
impl MinixFs {
    pub fn init() -> Result<()> {
        let mut global_fs = MINIX_FS.lock();

        const BLOCK_SIZE: usize = 1024;
        let size = 512 * 64;
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

    pub fn list_dir_entries(path: &[u8], buf: &mut [u8]) -> Result<usize> {
        let mut global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_mut()
            .ok_or(Failed("Minix filesystem is not initialized"))?;
        let root_inode = minix3_inode::new(0);

        let inode_num = root_inode.lookup_iter(path, &mut fs.image, fs.block_size)?;
        let inode_ptr = root_inode.get_inode(inode_num, fs.block_size, &mut fs.image)?;
        let inode = unsafe { core::ptr::read_unaligned(inode_ptr) };
        if (inode.i_mode & 0x4000) == 0 {
            return Err(Failed("path is not a directory"));
        }

        let entries = root_inode.read(&mut fs.image, path, fs.block_size)?;
        let mut written = 0;
        for entry_bytes in entries.chunks_exact(size_of::<minix3_dir_entry>()) {
            let entry = unsafe {
                core::ptr::read_unaligned(entry_bytes.as_ptr() as *const minix3_dir_entry)
            };
            if entry.inode == 0 {
                continue;
            }

            let name_len = entry
                .name
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(MINIX_MAX_FILENAME);
            let name = &entry.name[..name_len];
            if name == b"." || name == b".." {
                continue;
            }
            if written + name.len() + 1 > buf.len() {
                return Err(Failed("dir entry buffer is too small"));
            }

            buf[written..written + name.len()].copy_from_slice(name);
            written += name.len();
            buf[written] = b'\n';
            written += 1;
        }

        Ok(written)
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

    pub fn fs_img(buffer: &mut [u8]) -> Result<()> {
        let global_fs = MINIX_FS.lock();
        let fs = global_fs
            .as_ref()
            .ok_or(Failed("Minix filesystem is not initialized"))?;

        if buffer.len() != fs.image.len() {
            return Err(Failed("invalid buffer size for Minix filesystem image"));
        }

        buffer.copy_from_slice(fs.image.as_slice());
        Ok(())
    }
}
