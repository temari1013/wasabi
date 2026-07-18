use crate::error;
use crate::info;

macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            core::any::type_name::<T>()
        }
        let name = type_name_of(f);
        // Strips the anonymous function name ("::f") from the end
        name.strip_suffix("::f").unwrap()
    }};
}
pub type c_char = u8;

fn split_path_and_filename(file_path: &[u8]) -> (&[u8], &[u8]) {
    info!("Current function: {}", function_name!());
    let last_slash_idx = file_path.iter().rposition(|&c| c == b'/');

    match last_slash_idx {
        Some(idx) => {
            let parent = if idx == 0 { b"/" } else { &file_path[..idx] };
            let name = &file_path[idx + 1..];
            (parent, name)
        }
        None => (b"/", file_path),
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct minix3_super_block {
    pub s_ninodes: u32,
    pub s_pad0: u16,
    pub s_imap_blocks: u16,
    pub s_zmap_blocks: u16,
    pub s_firstdatazone: u16,
    pub s_log_zone_size: u16,
    pub s_pad1: u16,
    pub s_max_size: u32,
    pub s_zones: u32,
    pub s_magic: u16,
    pub s_pad2: u16,
    pub s_blocksize: u16,
    pub s_disk_version: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct minix3_dir_entry {
    pub inode: u32,
    pub name: [c_char; 60],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct minix3_inode {
    pub i_mode: u16,
    pub i_nlinks: u16,
    pub i_uid: u16,
    pub i_gid: u16,
    pub i_size: u32,
    pub i_atime: u32,
    pub i_mtime: u32,
    pub i_ctime: u32,
    pub i_zone: [u32; 10],
}
impl minix3_inode {
    pub fn get_inode(
        &self,
        inode_num: u32,
        block_size: usize,
        minix_img: &mut [u8],
    ) -> *mut minix3_inode {
        if inode_num == 0 {
            return core::ptr::null_mut();
        }

        let super_block = unsafe {
            *(minix_img.as_ptr().add(block_size) as *const minix3_super_block)
        };
        let inode_table_block = 2
            + super_block.s_imap_blocks as usize
            + super_block.s_zmap_blocks as usize;
        let base_offset = inode_table_block * block_size;
        let inode_idx =
            (inode_num - 1) as usize * core::mem::size_of::<minix3_inode>();
        let total_offset = base_offset + inode_idx;

        // info!("DEBUG: inode_table_block={}, base_offset={}, total_offset={}",
        // inode_table_block, base_offset, total_offset);

        unsafe { minix_img.as_mut_ptr().add(total_offset) as *mut minix3_inode }
    }

    pub fn new(mode: u16) -> minix3_inode {
        info!("Current function: {}", function_name!());
        let node = minix3_inode {
            i_mode: mode,
            i_nlinks: 0,
            i_uid: 0,
            i_gid: 0,
            i_size: 0,
            i_atime: 0,
            i_mtime: 0,
            i_ctime: 0,
            i_zone: [0; 10],
        };
        node
    }

    // 親ディレクトリ直下のファイルを名前一致で検索して、inodeの番号を返す
    pub fn lookup(
        &self,
        parent_inode: *mut minix3_inode,
        name: &[u8],
        minix_img: &[u8],
        block_size: usize,
    ) -> u32 {
        if (self.i_mode & 0x4000) == 0 {
            error!("invalid i_mode");
            return 0;
        }
        let parent_zone_block_num =
            unsafe { (*parent_inode).i_zone[0] as usize };

        let entries_count =
            block_size / core::mem::size_of::<minix3_dir_entry>();

        for i in 0..entries_count {
            let dir_entry = unsafe {
                *(minix_img
                    .as_ptr()
                    .add(parent_zone_block_num * block_size + i * 64)
                    as *const minix3_dir_entry)
            };

            let len = dir_entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &dir_entry.name[..len];
            if entry_name == name {
                return dir_entry.inode;
            }
        }
        0
    }

    pub fn lookup_iter(
        &self,
        path: &[u8],
        minix_img: &mut [u8],
        block_size: usize,
    ) -> u32 {
        info!("Current function: {}", function_name!());
        let mut current_inode_num = 1; // ルートからスタート

        for segment in path.split(|&c| c == b'/').filter(|s| !s.is_empty()) {
            let current_inode_ptr =
                self.get_inode(current_inode_num, block_size, minix_img);
            if current_inode_ptr.is_null() {
                return 0;
            }
            let current_inode = unsafe { &*current_inode_ptr };
            let next_inode_num = current_inode.lookup(
                current_inode_ptr,
                segment,
                minix_img,
                block_size,
            );

            if next_inode_num == 0 {
                return 0;
            }

            current_inode_num = next_inode_num;
        }

        current_inode_num
    }

    pub fn lookup_helper(
        &self,
        parent_inode: *const minix3_inode,
        name: &[u8],
        minix_img: &[u8],
        block_size: usize,
    ) -> u32 {
        info!("Current function: {}", function_name!());
        if (self.i_mode & 0x4000) == 0 {
            error!("invalid i_mode");
            return 0;
        }

        let parent_zone_block_num =
            unsafe { (*parent_inode).i_zone[0] as usize };

        for i in 2..32 {
            let dir_entry = unsafe {
                *(minix_img
                    .as_ptr()
                    .add(parent_zone_block_num * block_size + i * 64)
                    as *const minix3_dir_entry)
            };

            let len = dir_entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &dir_entry.name[..len];
            if entry_name == name {
                return dir_entry.inode;
            }
        }

        0
    }

    fn alloc_inode(
        &self,
        minix_img: &mut [u8],
        block_size: usize,
        mode: u16,
    ) -> u32 {
        let super_block = unsafe {
            *(minix_img.as_ptr().add(block_size) as *const minix3_super_block)
        };

        let imap_start_block = 2;
        let zmap_start_block =
            imap_start_block + super_block.s_imap_blocks as usize;
        let inode_table_block =
            zmap_start_block + super_block.s_zmap_blocks as usize;

        let max_bits = (super_block.s_imap_blocks as usize) * block_size * 8;
        let mut inode_num = 0;

        // inode番号0は無効
        for i in 1..max_bits {
            let bit_index = i - 1;
            let byte_idx = (imap_start_block * block_size) + (bit_index / 8);
            let bit_idx = bit_index % 8;

            if (minix_img[byte_idx] & (1u8 << bit_idx)) == 0 {
                inode_num = i;
                minix_img[byte_idx] |= 1u8 << bit_idx;
                break;
            }
        }

        if inode_num == 0 {
            error!("No free inode found");
            return 0;
        }

        let inode_offset = (inode_table_block * block_size)
            + ((inode_num - 1) * core::mem::size_of::<minix3_inode>());

        let node = Self::new(mode);
        unsafe {
            let target_ptr =
                minix_img.as_mut_ptr().add(inode_offset) as *mut minix3_inode;
            core::ptr::write(target_ptr, node);
        }
        inode_num as u32
    }
    fn link_inode(
        &self,
        minix_img: &mut [u8],
        parent_inode_num: u32,
        new_inode_num: u32,
        block_size: usize,
        file_name: &[u8],
    ) -> Result<(), bool> {
        let parent_inode_ptr =
            self.get_inode(parent_inode_num, block_size, minix_img);

        if parent_inode_ptr.is_null() {
            error!("link_inode: Parent inode is null");
            return Err(false);
        }

        let parent_inode = unsafe { &mut *parent_inode_ptr };
        let zone0_offset = parent_inode.i_zone[0] as usize * block_size;
        // info!("DEBUG: link_inode writing '{}' to offset {}",
        // core::str::from_utf8(file_name).unwrap_or("?"), zone0_offset);

        let entries_count =
            block_size / core::mem::size_of::<minix3_dir_entry>();

        let entries = unsafe {
            core::slice::from_raw_parts_mut(
                minix_img.as_mut_ptr().add(zone0_offset)
                    as *mut minix3_dir_entry,
                entries_count,
            )
        };

        for (i, entry) in entries.iter_mut().enumerate() {
            if entry.inode == 0 {
                entry.inode = new_inode_num;
                entry.name.fill(0);

                let copy_len = core::cmp::min(file_name.len(), 60);
                for i in 0..copy_len {
                    entry.name[i] = file_name[i];
                }
                // info!("DEBUG: link_inode success at entry index {}", i);
                return Ok(());
            }
        }
        Err(false)
    }

    pub fn create_file(
        &self,
        minix_img: &mut [u8],
        block_size: usize,
        path: &[u8],
    ) -> u32 {
        let mut inode_num = 0;
        info!("Current function: {}", function_name!());
        let (dir, filename) = split_path_and_filename(path);
        if dir == b"/" {
            inode_num = self.alloc_inode(minix_img, block_size, 0x0000)
        } else {
            let parent_inode_num = self.lookup_iter(dir, minix_img, block_size);
            if parent_inode_num == 0 {
                error!("Parent directory not found");
                return 0;
            }
            inode_num = self.alloc_inode(minix_img, block_size, 0x0000);
            let res = self.link_inode(
                minix_img,
                parent_inode_num,
                inode_num,
                block_size,
                filename,
            );
        }

        inode_num as u32
    }

    fn alloc_zone(
        &self,
        minix_img: &mut [u8],
        inode_num: u32,
        block_size: usize,
    ) -> u32 {
        let inode_ptr = self.get_inode(inode_num, block_size, minix_img);
        if inode_ptr.is_null() {
            crate::error!("Inode not found");
            return 0;
        }

        let super_block = unsafe {
            *(minix_img.as_ptr().add(block_size) as *const minix3_super_block)
        };

        let block_offset = 2 + super_block.s_imap_blocks as usize;
        let max_bits = (super_block.s_zmap_blocks as usize) * block_size * 8;

        // Minix zmap仕様
        // bit 1 が最初のデータゾーン(s_firstdatazone)に対応
        // つまり、bit j は「ゾーン j + s_firstdatazone - 1」を指す
        for j in 1..max_bits {
            let byte_idx = (block_offset * block_size) + (j / 8);
            let bit_idx = j % 8;
            let is_used = (minix_img[byte_idx] & (1u8 << bit_idx)) != 0;

            if !is_used {
                let allocated_zone =
                    j + (super_block.s_firstdatazone as usize) - 1;

                let inode = unsafe { &mut *inode_ptr };

                if inode.i_zone[0] == 0 {
                    // zmapを更新
                    minix_img[byte_idx] |= 1u8 << bit_idx;
                    inode.i_zone[0] = allocated_zone as u32;

                    // 新規割り当てされたデータブロックの初期化
                    let data_offset = allocated_zone * block_size;
                    if data_offset + block_size <= minix_img.len() {
                        minix_img[data_offset..data_offset + block_size]
                            .fill(0);
                    }

                    return allocated_zone as u32;
                } else {
                    crate::error!("i_zone[0] is already in use.");
                    return 0;
                }
            }
        }
        0
    }

    pub fn write(
        &self,
        minix_img: &mut [u8],
        file_path: &[u8],
        block_size: usize,
        data: &[u8],
    ) {
        info!("Current function: {}", function_name!());
        let inode_num = self.lookup_iter(file_path, minix_img, block_size);
        if inode_num == 0 {
            crate::error!("File not found for writing");
            return;
        }

        let inode = self.get_inode(inode_num, block_size, minix_img);
        if inode.is_null() {
            crate::error!("Inode is null");
            return;
        }

        let mut zone = unsafe { (*inode).i_zone[0] };

        if zone == 0 {
            zone = self.alloc_zone(minix_img, inode_num, block_size);
        }

        let zone_byte = zone as usize * block_size;
        let copy_len = data.len();

        unsafe {
            let dst_ptr = minix_img.as_mut_ptr().add(zone_byte);
            for i in 0..copy_len {
                core::ptr::write(dst_ptr.add(i), data[i]);
            }
            (*inode).i_size = copy_len as u32;
        }
    }

    pub fn mkdir(
        &self,
        minix_img: &mut [u8],
        path: &[u8],
        block_size: usize,
    ) -> u32 {
        info!("Current function: {}", function_name!());
        let (dir, dirname) = split_path_and_filename(path);

        let parent_inode_num = if dir == b"/" {
            1
        } else {
            self.lookup_iter(dir, minix_img, block_size)
        };
        if parent_inode_num == 0 {
            return 0;
        }

        let new_inode_num = self.alloc_inode(minix_img, block_size, 0x4000);
        if new_inode_num == 0 {
            return 0;
        }

        if self
            .link_inode(
                minix_img,
                parent_inode_num,
                new_inode_num,
                block_size,
                dirname,
            )
            .is_err()
        {
            return 0;
        }

        //  . と .. を作成
        let zone_num = self.alloc_zone(minix_img, new_inode_num, block_size);
        let zone_offset = zone_num as usize * block_size;
        let mut dot_name = [0u8; 60];
        dot_name[0] = b'.';
        let dot_entry = minix3_dir_entry {
            inode: new_inode_num,
            name: dot_name,
        };

        let mut dotdot_name = [0u8; 60];
        dotdot_name[0] = b'.';
        dotdot_name[1] = b'.';
        let dotdot_entry = minix3_dir_entry {
            inode: parent_inode_num,
            name: dotdot_name,
        };
        unsafe {
            let base = minix_img.as_mut_ptr().add(zone_offset)
                as *mut minix3_dir_entry;
            core::ptr::write(base, dot_entry);
            core::ptr::write(base.add(1), dotdot_entry);
        }
        new_inode_num
    }

    pub fn read<'a>(
        &self,
        minix_img: &'a mut [u8],
        file_path: &[u8],
        block_size: usize,
    ) -> &'a [u8] {
        info!("Current function: {}", function_name!());
        let inode_num = self.lookup_iter(file_path, minix_img, block_size);

        if inode_num == 0 {
            error!("File not found");
            return &[];
        }

        let inode = self.get_inode(inode_num, block_size, minix_img);
        if inode.is_null() {
            error!("Inode not found");
            return &[];
        }

        let (zone, size) =
            unsafe { ((*inode).i_zone[0], (*inode).i_size as usize) };

        if zone == 0 || size == 0 {
            return &[];
        }

        let start_offset = zone as usize * block_size;
        let end_offset = start_offset + size;

        if end_offset > minix_img.len() {
            error!("Read out of bounds");
            return &[];
        }

        &minix_img[start_offset..end_offset]
    }

    fn print_tree(
        &self,
        minix_img: &mut [u8],
        block_size: usize,
        depth: u8,
        inode_num: u32,
    ) {
        // info!("Current function: {}", function_name!());
        let inode_ptr = self.get_inode(inode_num, block_size, minix_img);
        if inode_ptr.is_null() {
            return;
        }
        let inode = unsafe { &*inode_ptr };

        if (inode.i_mode & 0x4000) == 0 {
            return;
        }
        let zone0_offset = inode.i_zone[0] as usize * block_size;
        if zone0_offset == 0 {
            return;
        }

        let entries_count =
            block_size / core::mem::size_of::<minix3_dir_entry>();
        let entries = unsafe {
            core::slice::from_raw_parts(
                minix_img.as_ptr().add(zone0_offset) as *const minix3_dir_entry,
                entries_count,
            )
        };

        for entry in entries {
            if entry.inode == 0 {
                continue;
            }

            let len = entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let name_slice = &entry.name[..len];

            // .と..をスキップ
            if name_slice == b"." || name_slice == b".." {
                continue;
            }
            let name_str =
                core::str::from_utf8(name_slice).unwrap_or("<invalid utf8>");
            let indent = [b' '; 32];
            let spaces = core::cmp::min(depth as usize * 2, 32);
            let indent_str =
                core::str::from_utf8(&indent[..spaces]).unwrap_or("");

            let child_inode_ptr =
                self.get_inode(entry.inode, block_size, minix_img);
            if child_inode_ptr.is_null() {
                continue;
            }
            let child_inode = unsafe { &*child_inode_ptr };

            if (child_inode.i_mode & 0x4000) != 0 {
                info!("{}|- {}/", indent_str, name_str);
                // ディレクトリであれば、深さを +1 して再帰呼び出し
                self.print_tree(minix_img, block_size, depth + 1, entry.inode);
            } else {
                info!("{}|- {}", indent_str, name_str);
            }
        }
    }

    pub fn show_directry_tree(&self, minix_img: &mut [u8], block_size: usize) {
        let root_ptr = self.get_inode(1, block_size, minix_img);
        if !root_ptr.is_null() {
            self.print_tree(minix_img, block_size, 1, 1);
        }
        info!("Current function: {}", function_name!());
        info!("/");
        self.print_tree(minix_img, block_size, 1, 1);
    }
}

pub fn delete() {}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::size_of;
    use core::ptr::read_unaligned;

    // 引数の型の関係でmutableとそうでないものの両方の読み込み方をする
    pub static MINIX3_IMG: [u8; 2097152] = *include_bytes!("minix3.img");
    pub static BLOCK_SIZE: usize = 1024;

    // inodeのメソッドとして実装しているのでinodeを取ってくる処理が必要
    fn get_root_inode_ptr_mut(
        img_ptr: *mut u8,
        block_size: usize,
    ) -> *mut minix3_inode {
        unsafe {
            let super_block_ptr =
                img_ptr.add(block_size) as *const minix3_super_block;
            let super_block = read_unaligned(super_block_ptr);
            let inode_table_block = 2
                + super_block.s_imap_blocks as usize
                + super_block.s_zmap_blocks as usize;

            let base_offset = inode_table_block * block_size;

            img_ptr.add(base_offset) as *mut minix3_inode
        }
    }

    fn get_root_inode_ptr(
        img_ptr: *const u8,
        block_size: usize,
    ) -> *const minix3_inode {
        unsafe {
            let super_block_ptr =
                img_ptr.add(block_size) as *const minix3_super_block;
            let super_block = read_unaligned(super_block_ptr);
            let inode_table_block = 2
                + super_block.s_imap_blocks as usize
                + super_block.s_zmap_blocks as usize;

            let base_offset = inode_table_block * block_size;

            img_ptr.add(base_offset) as *const minix3_inode
        }
    }

    #[test_case]
    fn file_create_test() {
        // ファイルを作成し、想定通りに作成できているかを見る
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();

        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        root_inode.create_file(&mut minix_img, BLOCK_SIZE, b"dir/newfile.txt");
    }

    #[test_case]
    fn file_write_test() {
        // すでにイメージファイルにあるものに書き込み、
        // 想定通りに書き込めているかを見る
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();

        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        root_inode.write(
            &mut minix_img,
            b"/dir/nested/syouyu.txt",
            BLOCK_SIZE,
            b"test",
        );
    }

    #[test_case]
    fn lookup_inode_iter_test() {
        //  lookup_inodeにroodeのポインタと探すディレクトリ、
        // その他引数を渡すと想定通りの番号が帰ってくることを期待する。
        let img_ptr = MINIX3_IMG.as_ptr();

        let root_inode_ptr = get_root_inode_ptr(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        let target_inode_num = root_inode.lookup(
            root_inode_ptr as *mut minix3_inode,
            b"dir",
            &MINIX3_IMG,
            BLOCK_SIZE,
        );
        assert_eq!(target_inode_num, 2);
    }
}
