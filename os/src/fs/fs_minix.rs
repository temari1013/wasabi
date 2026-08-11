extern crate alloc;

use core::unimplemented;

use crate::efi::fs;
use crate::error::Error::Failed;
use crate::error::Result;
use crate::fs::minix_manager::MinixFs;
use alloc::rc::Rc;
use alloc::vec::Vec;
// プロセスに割り当てられる開かれたファイルの構造体
pub struct File {
    pub count: u8,
    pub path: Vec<u8>,
    pub fmode_t: u8,
    pub inode_num: u32,
    pub ptr: u8, //ファイル構造体のポインタではなく、ファイル内の見ている位置のポインタ
}
impl File {
    pub fn new() -> File {
        File {
            count: 0,
            path: Vec::new(),
            fmode_t: 0,
            inode_num: 0,
            ptr: 0,
        }
    }
    pub fn open(path: &[u8], fmode_t: u8, ptr: u8) -> Result<Rc<File>> {
        // パスを見て存在チェック
        let inode_num = MinixFs::lookup_iter(path)?;
        let mut file = File::new();

        // 存在しない場合、モードに応じてエラーもしくは新規作成
        // 一旦存在しない場合はエラー
        if inode_num == 0 {
            return Err(Failed(
                "failed to open the file. please check file existence",
            ));
        }

        //
        file.fmode_t = fmode_t;
        file.path = path.to_vec();
        file.inode_num = inode_num;
        file.ptr = ptr;

        let file = Rc::new(file);
        Ok(file)
    }

    pub fn close(&mut self) -> Result<i64> {
        unimplemented!();
    }
}
