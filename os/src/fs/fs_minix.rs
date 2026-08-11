extern crate alloc;

use crate::error::Result;
use alloc::rc::Rc;
use alloc::vec::Vec;
// プロセスに割り当てられる開かれたファイルの構造体
pub struct File {
    pub path: Vec<u8>,
    pub fmode_t: u8,
    pub ptr: u8,
}
impl File {
    pub fn new() -> File {
        File {
            path: Vec::new(),
            fmode_t: 0,
            ptr: 0,
        }
    }
    pub fn open(&mut self, path: Vec<u8>, fmode_t: u8, ptr: u8) -> Result<Rc<File>> {
        //　パスを見て存在チェック
        // 存在しない場合、モードに応じてエラーもしくは新規作成
        // 解決もしくは作成したファイルを元に、File構造体を作成する
    }
}
