extern crate alloc;
use crate::error;
use crate::error::Error::Failed;
use crate::error::Result;
use crate::info;
use core::mem::size_of;

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
pub const MINIX_MAX_FILENAME: usize = 60;

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
impl minix3_super_block {
    pub fn new(block_size: usize) -> minix3_super_block {
        let super_block = minix3_super_block {
            s_ninodes: 0,
            s_pad0: 0,
            s_imap_blocks: 0,
            s_zmap_blocks: 0,
            s_firstdatazone: 0,
            s_log_zone_size: 0,
            s_pad1: 0,
            s_max_size: 0,
            s_zones: 0,
            s_magic: 0x4D5A,
            s_pad2: 0,
            s_blocksize: block_size as u16,
            s_disk_version: 0,
        };
        super_block
    }
    fn imap_start_block(&self) -> usize {
        2
    }
    fn zmap_start_block(&self) -> usize {
        self.imap_start_block() + self.s_imap_blocks as usize
    }
    fn inode_table_start_block(&self) -> usize {
        self.zmap_start_block() + self.s_zmap_blocks as usize
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct minix3_dir_entry {
    pub inode: u32,
    pub name: [c_char; MINIX_MAX_FILENAME],
}
impl minix3_dir_entry {
    pub fn new(inode: u32, name: &[u8]) -> minix3_dir_entry {
        let mut entry_name = [0 as u8; 60];
        let len = core::cmp::min(name.len(), 60);
        entry_name[..len].copy_from_slice(&name[..len]);

        minix3_dir_entry {
            inode,
            name: entry_name,
        }
    }
    fn get_entries(
        minix_img: &mut [u8],
        zone_block_num: u32,
        block_size: usize,
        entries_count: u32,
    ) -> Result<&mut [minix3_dir_entry]> {
        // そのゾーンに含まれるエントリの配列を返す
        // SAFETY : zone_block_numが有効なブロック番号であり、minix_imgが十分な長さを持つことは呼び出し元の責任
        let entries = unsafe {
            core::slice::from_raw_parts_mut(
                minix_img
                    .as_mut_ptr()
                    .add(zone_block_num as usize * block_size)
                    as *mut minix3_dir_entry,
                entries_count as usize,
            )
        };
        Ok(entries)
    }

    fn as_bytes(&self) -> &[u8] {
        // SAFETY : selfは有効なminix3_dir_entryである
        unsafe {
            core::slice::from_raw_parts(
                (self as *const minix3_dir_entry) as *const u8,
                core::mem::size_of::<minix3_dir_entry>(),
            )
        }
    }
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
    fn as_bytes(&self) -> &[u8] {
        // SAFETY : selfは有効なminix3_inodeである
        unsafe {
            core::slice::from_raw_parts(
                (self as *const minix3_inode) as *const u8,
                core::mem::size_of::<minix3_inode>(),
            )
        }
    }

    pub fn get_inode(
        &self,
        inode_num: u32,
        block_size: usize,
        minix_img: &mut [u8],
    ) -> Result<*mut minix3_inode> {
        if inode_num == 0 {
            return Err(Failed("inode_num is 0"));
        }
        let super_block = get_super_block(minix_img, block_size);
        let base_offset = super_block.inode_table_start_block() * block_size;
        // Subtract 1 to align the 1-based inode number with the 0-based array index.
        let inode_idx = (inode_num - 1) as usize * core::mem::size_of::<minix3_inode>();
        let total_offset = base_offset + inode_idx;
        if total_offset + size_of::<minix3_inode>() > minix_img.len() {
            return Err(Failed(
                " total offset + base offset is longer than image length",
            ));
        }

        Ok(minix_img[total_offset..].as_mut_ptr() as *mut minix3_inode)
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
    pub fn lookup(&self, name: &[u8], minix_img: &[u8], block_size: usize) -> Result<u32> {
        if (self.i_mode & 0x4000) == 0 {
            error!("invalid i_mode");
            return Err(Failed("invalie i_mode"));
        }
        // SAFETY : parent_inodeは有効な引数である
        let parent_zone_block_num = self.i_zone[0] as usize;
        let entries_count = block_size / core::mem::size_of::<minix3_dir_entry>();

        for i in 0..entries_count {
            // SAFETY : entries_countの範囲内でアクセスしているので、minix_imgの有効範囲内である
            let dir_entry = unsafe {
                *(minix_img
                    .as_ptr()
                    .add(parent_zone_block_num * block_size + i * size_of::<minix3_dir_entry>())
                    as *const minix3_dir_entry)
            };
            let len = dir_entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &dir_entry.name[..len];
            if entry_name == name {
                return Ok(dir_entry.inode);
            }
        }
        return Err(Failed("lookup failed"));
    }

    pub fn lookup_iter(&self, path: &[u8], minix_img: &mut [u8], block_size: usize) -> Result<u32> {
        info!("Current function: {}", function_name!());
        let mut current_inode_num = 1; // ルートからスタート

        for segment in path.split(|&c| c == b'/').filter(|s| !s.is_empty()) {
            let current_inode_ptr = self.get_inode(current_inode_num, block_size, minix_img)?;
            // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
            let current_inode = unsafe { &*current_inode_ptr };
            let next_inode_num = current_inode.lookup(segment, minix_img, block_size)?;
            current_inode_num = next_inode_num;
        }
        Ok(current_inode_num)
    }

    fn alloc_inode(&self, minix_img: &mut [u8], block_size: usize, mode: u16) -> Result<u32> {
        info!("Current function: {}", function_name!());
        let mut inode_num = 0;
        let super_block = get_super_block(minix_img, block_size);
        let max_bits = (super_block.s_imap_blocks as usize) * block_size * 8;

        // inode番号0は無効なので1から見る
        for i in 1..max_bits {
            let bit_index = i - 1;
            let byte_idx = (super_block.imap_start_block() * block_size) + (bit_index / 8);
            let bit_idx = bit_index % 8;

            if (minix_img[byte_idx] & (1u8 << bit_idx)) == 0 {
                inode_num = i;
                change_i_bitmap(minix_img, block_size, bit_index, 1);
                break;
            }
        }
        if inode_num == 0 {
            error!("No free inode found");
            return Err(Failed("failed to alloc inode"));
        }

        let inode_offset = (super_block.inode_table_start_block() * block_size)
            + ((inode_num - 1) * core::mem::size_of::<minix3_inode>());

        let node = Self::new(mode);
        let target = &mut minix_img[inode_offset..inode_offset + size_of::<minix3_inode>()];

        target.copy_from_slice(node.as_bytes());

        Ok(inode_num as u32)
    }

    fn link_inode(
        &self,
        minix_img: &mut [u8],
        parent_inode_num: u32,
        new_inode_num: u32,
        block_size: usize,
        file_name: &[u8],
    ) -> Result<()> {
        let parent_inode_ptr = self.get_inode(parent_inode_num, block_size, minix_img)?;
        // SAFETY : get_inodeからポインタが帰ってきているなら有効
        let parent_inode = unsafe { &mut *parent_inode_ptr };

        let entries_count = block_size / core::mem::size_of::<minix3_dir_entry>();
        let entries = minix3_dir_entry::get_entries(
            minix_img,
            parent_inode.i_zone[0],
            block_size,
            entries_count as u32,
        )?;

        for (_, entry) in entries.iter_mut().enumerate() {
            if entry.inode == 0 {
                entry.inode = new_inode_num;
                entry.name.fill(0);

                let copy_len = core::cmp::min(file_name.len(), 60);
                for i in 0..copy_len {
                    entry.name[i] = file_name[i];
                }
                return Ok(());
            }
        }
        return Err(Failed("link inode failed"));
    }

    pub fn create_file(&self, minix_img: &mut [u8], block_size: usize, path: &[u8]) -> Result<u32> {
        let mut inode_num = 0;
        info!("Current function: {}", function_name!());
        let (dir, filename) = split_path_and_filename(path);
        if dir == b"/" {
            inode_num = self.alloc_inode(minix_img, block_size, 0x0000)?;
            let _ = self.link_inode(minix_img, 1, inode_num, block_size, filename);
        } else {
            let parent_inode_num = self.lookup_iter(dir, minix_img, block_size)?;

            inode_num = self.alloc_inode(minix_img, block_size, 0x0000)?;
            info!("{}", inode_num);
            self.link_inode(minix_img, parent_inode_num, inode_num, block_size, filename)?;
        }

        Ok(inode_num as u32)
    }

    fn alloc_zone(&self, minix_img: &mut [u8], inode_num: u32, block_size: usize) -> Result<u32> {
        let inode_ptr = self.get_inode(inode_num, block_size, minix_img)?;
        let super_block = get_super_block(minix_img, block_size);

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
                let allocated_zone = j + (super_block.s_firstdatazone as usize) - 1;
                // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
                let inode = unsafe { &mut *inode_ptr };

                if inode.i_zone[0] == 0 {
                    // zmapを更新
                    minix_img[byte_idx] |= 1u8 << bit_idx;
                    inode.i_zone[0] = allocated_zone as u32;

                    // 新規割り当てされたデータブロックの初期化
                    let data_offset = allocated_zone * block_size;
                    if data_offset + block_size <= minix_img.len() {
                        minix_img[data_offset..data_offset + block_size].fill(0);
                    }

                    return Ok(allocated_zone as u32);
                } else {
                    return Err(Failed("alloc zone failed"));
                }
            }
        }
        return Ok(0 as u32);
    }

    pub fn write(
        &self,
        minix_img: &mut [u8],
        file_path: &[u8],
        block_size: usize,
        data: &[u8],
    ) -> Result<()> {
        info!("Current function: {}", function_name!());
        let inode_num = self.lookup_iter(file_path, minix_img, block_size)?;
        let inode = self.get_inode(inode_num, block_size, minix_img)?;
        // SAFETY: get_inodeからポインタが帰ってきているならそれは有効
        let mut zone = unsafe { (*inode).i_zone[0] };

        // 書き込み時にzoneが存在しない場合新規にアロックする
        if zone == 0 {
            zone = self.alloc_zone(minix_img, inode_num, block_size)?;
        }

        let zone_byte = zone as usize * block_size;
        let copy_len = data.len();

        // SAFETY: TODO
        // 多分unsafe使用しなくても書ける

        unsafe {
            let dst_ptr = minix_img.as_mut_ptr().add(zone_byte);
            for i in 0..copy_len {
                core::ptr::write(dst_ptr.add(i), data[i]);
            }
            (*inode).i_size = copy_len as u32;
        }
        Ok(())
    }

    pub fn mkdir(&self, minix_img: &mut [u8], path: &[u8], block_size: usize) -> Result<u32> {
        info!("Current function: {}", function_name!());
        let (dir, dirname) = split_path_and_filename(path);

        let parent_inode_num = if dir == b"/" {
            1
        } else {
            self.lookup_iter(dir, minix_img, block_size)?
        };
        if parent_inode_num == 0 {
            return Err(Failed("parent inode num is 0"));
        }

        let new_inode_num = self.alloc_inode(minix_img, block_size, 0x4000)?;
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
            return Err(Failed("link inode failed"));
        }

        //  . と .. を作成
        let zone_offset =
            self.alloc_zone(minix_img, new_inode_num, block_size)? as usize * block_size;

        let dot_entry = minix3_dir_entry::new(new_inode_num, b".");
        let dotdot_entry = minix3_dir_entry::new(parent_inode_num, b"..");

        let zone = &mut minix_img[zone_offset..zone_offset + block_size];

        zone[0..size_of::<minix3_dir_entry>()].copy_from_slice(dot_entry.as_bytes());
        zone[size_of::<minix3_dir_entry>()..size_of::<minix3_dir_entry>() * 2]
            .copy_from_slice(dotdot_entry.as_bytes());

        let new_inode_ptr = self.get_inode(new_inode_num, block_size, minix_img)?;

        // SAFETY: get_inodeがエラーでないなら有効なポインタ
        unsafe {
            (*new_inode_ptr).i_size = size_of::<minix3_dir_entry>() as u32 * 2;
        }

        Ok(new_inode_num)
    }

    pub fn read<'a>(
        &self,
        minix_img: &'a mut [u8],
        file_path: &[u8],
        block_size: usize,
    ) -> Result<&'a [u8]> {
        info!("Current function: {}", function_name!());
        let inode_num = self.lookup_iter(file_path, minix_img, block_size)?;
        let inode = self.get_inode(inode_num, block_size, minix_img)?;
        // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
        let (zone, size) = unsafe { ((*inode).i_zone[0], (*inode).i_size as usize) };

        if zone == 0 || size == 0 {
            return Ok(&[]);
        }

        let start_offset = zone as usize * block_size;
        let end_offset = start_offset + size;

        if end_offset > minix_img.len() {
            error!("Read out of bounds");
            return Ok(&[]);
        }

        Ok(&minix_img[start_offset..end_offset])
    }

    fn print_tree(
        &self,
        minix_img: &mut [u8],
        block_size: usize,
        depth: u8,
        inode_num: u32,
    ) -> Result<()> {
        // info!("Current function: {}", function_name!());
        let inode_ptr = self.get_inode(inode_num, block_size, minix_img)?;
        // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
        let inode = unsafe { &*inode_ptr };

        if (inode.i_mode & 0x4000) == 0 {
            return Err(Failed("imode is invalid"));
        }
        let zone0_offset = inode.i_zone[0] as usize * block_size;
        if zone0_offset == 0 {
            return Err(Failed("zone offset is invalid"));
        }

        let entries_count = block_size / core::mem::size_of::<minix3_dir_entry>();
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
            let name_str = core::str::from_utf8(name_slice).unwrap_or("<invalid utf8>");
            let indent = [b' '; 32];
            let spaces = core::cmp::min(depth as usize * 2, 32);
            let indent_str = core::str::from_utf8(&indent[..spaces]).unwrap_or("");

            let child_inode_ptr = self.get_inode(entry.inode, block_size, minix_img)?;
            // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
            let child_inode = unsafe { &*child_inode_ptr };

            if (child_inode.i_mode & 0x4000) != 0 {
                info!("{}|- {}/", indent_str, name_str);
                // ディレクトリであれば、深さを +1 して再帰呼び出し
                self.print_tree(minix_img, block_size, depth + 1, entry.inode)?;
            } else {
                info!("{}|- {}", indent_str, name_str);
            }
        }
        Ok(())
    }

    pub fn show_directory_tree(&self, minix_img: &mut [u8], block_size: usize) -> Result<()> {
        info!("Current function: {}", function_name!());
        info!("/");
        self.print_tree(minix_img, block_size, 1, 1)?;
        return Ok(());
    }

    // 末端ファイルもしくはディレクトリの消去
    pub fn delete_dir_entry(
        &self,
        minix_img: &mut [u8],
        file_path: &[u8],
        block_size: usize,
    ) -> Result<()> {
        info!("Current function: {}", function_name!());
        let super_block = get_super_block(minix_img, block_size);

        // パスを親ディレクトリと本人に分割
        let (parent_dir, filename) = split_path_and_filename(file_path);

        let inode_num = self.lookup_iter(file_path, minix_img, block_size)?;
        let inode_ptr = self.get_inode(inode_num, block_size, minix_img)?;

        // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
        let inode = unsafe { *(inode_ptr as *const minix3_inode) };

        if (inode.i_mode & 0x4000) != 0 {
            // ディレクトリであれば中身が空でないと消去できない
            if inode.i_size != 128 {
                return Err(Failed("directory is not empty"));
            }
        }

        let zone_block_num = inode.i_zone[0] as usize;
        if zone_block_num != 0 {
            let zone_offset = zone_block_num * block_size;
            if zone_offset + block_size <= minix_img.len() {
                minix_img[zone_offset..zone_offset + block_size].fill(0);
            }
            // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
            unsafe {
                (*inode_ptr).i_zone[0] = 0;
                (*inode_ptr).i_size = 0;
            }
        }
        // imapの更新
        let imap_start_block = super_block.imap_start_block();
        let bit_index = inode_num as usize - 1; // inode番号は1から始まるので、0-basedに変換
        let byte_idx = (imap_start_block * block_size) + (bit_index / 8);
        let bit_idx = bit_index % 8;
        minix_img[byte_idx] &= !(1u8 << bit_idx);

        // zmapの更新(割り当てられている場合のみ)
        if inode.i_zone[0] > 0 {
            let zmap_start_block = super_block.zmap_start_block();
            let firstdatazone = super_block.s_firstdatazone as usize;
            let bit_index = inode.i_zone[0] as usize - firstdatazone + 1;
            let byte_idx = (zmap_start_block * block_size) + (bit_index / 8);
            let bit_idx = bit_index % 8;
            minix_img[byte_idx] &= !(1u8 << bit_idx);
        }

        // 親のエントリから自身を消す
        let parent_inode_num = if parent_dir == b"/" {
            1
        } else {
            self.lookup_iter(parent_dir, minix_img, block_size)?
        };
        if parent_inode_num == 0 {
            return Err(Failed("parent inode is 0"));
        }

        let parent_inode_ptr = self.get_inode(parent_inode_num, block_size, minix_img)?;
        // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
        let parent_zone_block_num = unsafe { (*parent_inode_ptr).i_zone[0] as usize };
        if parent_zone_block_num == 0 {
            return Err(Failed("parent zone block number is 0"));
        }

        let entries_count = block_size / core::mem::size_of::<minix3_dir_entry>();
        let entries = minix3_dir_entry::get_entries(
            minix_img,
            parent_zone_block_num as u32,
            block_size,
            entries_count as u32,
        )?;

        for entry in entries.iter_mut() {
            if entry.inode == inode_num {
                let len = entry.name.iter().position(|&c| c == 0).unwrap_or(60);
                let entry_name = &entry.name[..len];
                if entry_name == filename {
                    entry.inode = 0;
                    entry.name.fill(0);
                    break;
                }
            }
        }
        Ok(())
    }
    // alloc_inodeとalloc_zoneが割り当て時に初期化するので、データ部自体の0埋めは必要ない
}

pub fn init_minixfs(mem: &mut [u8], block_size: usize) {
    info!("Current function: {}", function_name!());

    let fs_size: usize = mem.len();
    let mut super_block = minix3_super_block::new(block_size);

    // zoneに何ブロック使えるかを考える
    let total_blocks = fs_size / block_size as usize;
    // inode比率は8個に一個とする
    let s_ninodes = (total_blocks / 8) as u32;

    let inodes_per_block = block_size as u32 / 64;
    // 切り上げとして処理するための足し算が入る
    let inode_table_blocks = (s_ninodes + inodes_per_block - 1) / inodes_per_block;
    let bits_per_block = block_size as u32 * 8;
    let imap_blocks = (s_ninodes + bits_per_block - 1) / bits_per_block;
    let zmap_blocks = (total_blocks + bits_per_block as usize - 1) / bits_per_block as usize;

    let firstdatazone = 2 + imap_blocks as u32 + zmap_blocks as u32 + inode_table_blocks;

    // スーパーブロックの初期化
    super_block.s_ninodes = s_ninodes;
    super_block.s_imap_blocks = imap_blocks as u16;
    super_block.s_zmap_blocks = zmap_blocks as u16;
    super_block.s_firstdatazone = firstdatazone as u16;
    super_block.s_zones = total_blocks as u32;

    unsafe {
        let mem_ptr = mem.as_mut_ptr().add(1024) as *mut minix3_super_block;
        core::ptr::write_unaligned(mem_ptr, super_block);
    }

    // iとzのbitmapを0埋めする（ゾーンブロックは割り当て時に初期化があるので放置でok)
    let begin = 2 * block_size as u32;
    let end = (2 as u32 + imap_blocks as u32 + zmap_blocks as u32) * block_size as u32;
    mem[begin as usize..end as usize].fill(0);

    // rootのinodeを書き込む
    let mut root_inode = minix3_inode::new(0x4000);
    root_inode.i_size = 128;
    root_inode.i_zone[0] = firstdatazone;
    unsafe {
        let mem_ptr = mem.as_mut_ptr().add(end as usize) as *mut minix3_inode;
        core::ptr::write_unaligned(mem_ptr, root_inode);
    }

    // すでに使われているmapの領域は1に戻す
    change_i_bitmap(mem, block_size as usize, 0, 1);
    change_i_bitmap(mem, block_size as usize, 1, 1);

    change_z_bitmap(mem, block_size, 0, 1);
    change_z_bitmap(mem, block_size, 1, 1);

    // . と .. の配置
    let entry_dot = minix3_dir_entry::new(1, b".");
    let entry_dotdot = minix3_dir_entry::new(1, b"..");
    let data_offset = firstdatazone as usize * block_size as usize;

    let target = &mut mem[data_offset..data_offset + 2 * size_of::<minix3_dir_entry>()];
    target[..size_of::<minix3_dir_entry>()].copy_from_slice(entry_dot.as_bytes());
    target[size_of::<minix3_dir_entry>()..2 * size_of::<minix3_dir_entry>()]
        .copy_from_slice(entry_dotdot.as_bytes());
}

pub fn get_super_block(minix_img: &mut [u8], block_size: usize) -> minix3_super_block {
    // SAFETY : The second block of the Minix image is the superblock.
    let super_block = unsafe { *(minix_img.as_ptr().add(block_size) as *const minix3_super_block) };
    super_block
}

fn change_i_bitmap(minix_img: &mut [u8], block_size: usize, bit_index: usize, value: u8) {
    let start_block = get_super_block(minix_img, block_size).imap_start_block();
    let byte_idx = (start_block * block_size) + (bit_index / 8);
    let bit_idx = bit_index % 8;

    if value == 1 {
        minix_img[byte_idx] |= 1u8 << bit_idx;
    } else {
        minix_img[byte_idx] &= !(1u8 << bit_idx);
    }
}
fn change_z_bitmap(minix_img: &mut [u8], block_size: usize, bit_index: usize, value: u8) {
    let start_block = get_super_block(minix_img, block_size).zmap_start_block();
    let byte_idx = (start_block * block_size) + (bit_index / 8);
    let bit_idx = bit_index % 8;

    if value == 1 {
        minix_img[byte_idx] |= 1u8 << bit_idx;
    } else {
        minix_img[byte_idx] &= !(1u8 << bit_idx);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use core::assert_eq;
    use core::ptr::read_unaligned;

    pub static MINIX3_IMG: [u8; 2097152] = *include_bytes!("minix3.img");
    pub static BLOCK_SIZE: usize = 1024;

    fn get_root_inode_ptr_mut(img_ptr: *mut u8, block_size: usize) -> *mut minix3_inode {
        unsafe {
            let super_block_ptr = img_ptr.add(block_size) as *const minix3_super_block;
            let super_block = read_unaligned(super_block_ptr);
            let inode_table_block =
                2 + super_block.s_imap_blocks as usize + super_block.s_zmap_blocks as usize;

            let base_offset = inode_table_block * block_size;

            img_ptr.add(base_offset) as *mut minix3_inode
        }
    }

    fn get_root_inode_ptr(img_ptr: *const u8, block_size: usize) -> *const minix3_inode {
        unsafe {
            let super_block_ptr = img_ptr.add(block_size) as *const minix3_super_block;
            let super_block = read_unaligned(super_block_ptr);
            let inode_table_block =
                2 + super_block.s_imap_blocks as usize + super_block.s_zmap_blocks as usize;

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

        // 確認すべきは親ディレクトリのzone[0]にnewfile.
        // txtがファイル名のディレクトリエントリがあるかどうか
        let offset = 48 * BLOCK_SIZE;

        let entries_count = BLOCK_SIZE / core::mem::size_of::<minix3_dir_entry>();
        let mut assertion = false;
        for i in 0..entries_count {
            let dir_entry =
                unsafe { *(minix_img.as_ptr().add(offset + i * 64) as *const minix3_dir_entry) };

            let len = dir_entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &dir_entry.name[..len];
            if entry_name == b"newfile.txt" {
                // TODO : 紐づいているinodeを確認する
                let inode = dir_entry.inode;
                assertion = true;
                break;
            }
        }
        assert!(assertion == true);
    }

    #[test_case]
    fn read_test() {
        let img_ptr = MINIX3_IMG.as_ptr();

        let root_inode_ptr = get_root_inode_ptr(img_ptr, BLOCK_SIZE);
        // SAFETY : get_inodeからポインタが帰ってきているならそれは有効
        let root_inode = unsafe { core::ptr::read_unaligned(root_inode_ptr) };

        let data = root_inode
            .read(
                unsafe { &mut *(img_ptr as *mut [u8; 2097152]) },
                b"dir/test.txt",
                BLOCK_SIZE,
            )
            .unwrap();

        let expected = b"hello";
        assert!(data.len() >= expected.len());
        assert_eq!(&data[..expected.len()], expected);
    }

    #[test_case]
    fn mkdir_test() {
        // ディレクトリを作成し、想定通りに作成できているかを見る
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();

        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        root_inode.mkdir(&mut minix_img, b"dir/nested/new_dir", BLOCK_SIZE);

        // 親ディレクトリに作成した名前のディレクトリエントリが配置されているかどうかとinodeの値の検証
        let offset = 49 * BLOCK_SIZE;

        let entries_count = BLOCK_SIZE / core::mem::size_of::<minix3_dir_entry>();
        let mut assertion = false;
        for i in 0..entries_count {
            let dir_entry =
                unsafe { *(minix_img.as_ptr().add(offset + i * 64) as *const minix3_dir_entry) };

            let len = dir_entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &dir_entry.name[..len];
            if entry_name == b"new_dir" {
                assertion = true;
                break;
            }
        }
        assert!(assertion == true);
    }

    #[test_case]
    fn file_write_test() {
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();

        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        root_inode.write(
            &mut minix_img,
            b"/dir/nested/syouyu.txt",
            BLOCK_SIZE,
            b"syouyu",
        );

        // Verify
        let offset = 52 * BLOCK_SIZE;
        let written_data = &minix_img[offset..offset + 6];

        assert_eq!(written_data, b"syouyu")
    }

    #[test_case]
    fn lookup_inode_iter_test() {
        //  lookup_inodeにroodeのポインタと探すディレクトリ、
        // その他引数を渡すと想定通りの番号が帰ってくることを期待する。
        let img_ptr = MINIX3_IMG.as_ptr();

        let root_inode_ptr = get_root_inode_ptr(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        let target_inode_num = root_inode.lookup(b"dir", &MINIX3_IMG, BLOCK_SIZE).unwrap();
        assert_eq!(target_inode_num, 2);
    }

    #[test_case]
    fn check() {
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();
        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        let inode_num = root_inode
            .lookup_iter(b"/dir/test.txt", &mut minix_img, BLOCK_SIZE)
            .unwrap();

        let zone_num = root_inode
            .get_inode(inode_num, BLOCK_SIZE, &mut minix_img)
            .unwrap();
        let zone_num = unsafe { (*zone_num).i_zone[0] };
        info!("inode_num: {}, zone_num: {}", inode_num, zone_num);
    }

    #[test_case]
    fn delete_test() {
        // delete_dir_entryを呼ぶと、想定通りにディレクトリエントリが削除されることを期待する
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();
        let super_block_ptr = minix_img[BLOCK_SIZE..].as_ptr() as *const minix3_super_block;
        let super_block = unsafe { core::ptr::read_unaligned(super_block_ptr) };
        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        // 自身のinodeのビットマップが１
        let inode_num = 3;
        let inode_byte_offset = (inode_num / 8) as usize;
        let inode_bit_offset = inode_num % 8;

        let imap_base = 2 * BLOCK_SIZE;

        let target_imap_byte_before = minix_img[imap_base + inode_byte_offset];
        let is_inode_used_before = (target_imap_byte_before & (1 << inode_bit_offset)) != 0;

        assert!(is_inode_used_before, "inode bitmap is not set");

        // test.txtの存在するzone_num = 50 のビットマップが1
        let firstdatazone = super_block.s_firstdatazone as usize;
        let zmap_bit_index = 50 - firstdatazone + 1;

        let zmap_base = super_block.zmap_start_block() as usize * BLOCK_SIZE;
        let zmap_byte_offset = zmap_bit_index / 8;
        let zmap_bit_offset = zmap_bit_index % 8;

        let target_zmap_byte_before = minix_img[zmap_base + zmap_byte_offset];
        let is_zone_used_before = (target_zmap_byte_before & (1 << zmap_bit_offset)) != 0;
        assert!(
            is_zone_used_before,
            "zone bitmap is not set before deletion"
        );

        root_inode
            .delete_dir_entry(&mut minix_img, b"/dir/test.txt", BLOCK_SIZE)
            .unwrap();

        // 自身のinodeのビットマップが0になっている
        let target_imap_byte_after = minix_img[imap_base + inode_byte_offset];
        let is_inode_used_after = (target_imap_byte_after & (1 << inode_bit_offset)) != 0;
        assert!(!is_inode_used_after, "inode bitmap is not cleared");

        // 親ディレクトリエントリのzone[0]に自身が存在しない
        let parent_dir_offset = 2 * BLOCK_SIZE;

        let entries_count = BLOCK_SIZE / core::mem::size_of::<minix3_dir_entry>();
        let mut entry_exists = false;
        for i in 0..entries_count {
            let dir_entry = unsafe {
                *(minix_img.as_ptr().add(parent_dir_offset + i * 64) as *const minix3_dir_entry)
            };

            let len = dir_entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &dir_entry.name[..len];
            if entry_name == b"test.txt" {
                entry_exists = true;
                break;
            }
        }
        assert!(entry_exists == false, "directory entry still exists");

        // test.txtの存在するzone_num = 50 のビットマップが0
        let target_zmap_byte_after = minix_img[zmap_base + zmap_byte_offset];
        let is_zone_used_after = (target_zmap_byte_after & (1 << zmap_bit_offset)) != 0;
        assert!(!is_zone_used_after, "zone bitmap is not cleared");
    }

    #[test_case]
    fn delete_test2() {
        // 中が存在するディレクトリは削除できない
        let mut minix_img = include_bytes!("minix3.img").to_vec();
        let img_ptr = minix_img.as_mut_ptr();
        let root_inode_ptr = get_root_inode_ptr_mut(img_ptr, BLOCK_SIZE);
        let root_inode = unsafe { read_unaligned(root_inode_ptr) };

        let result = root_inode.delete_dir_entry(&mut minix_img, b"/dir", BLOCK_SIZE);
        assert!(
            result.is_err(),
            "Expected error when deleting non-empty directory"
        );
    }

    #[test_case]
    fn minix_init_test() {
        // init_minixfsを呼ぶと、想定通りに初期化されることを期待する
        const BLOCK_SIZE: usize = 1024;
        let size = 4 * 1024 * 1024;
        let mut mem = alloc::vec![0u8; size];

        init_minixfs(&mut mem, BLOCK_SIZE);

        // superblockに想定通りの値が入っている
        let super_block_ptr = mem[BLOCK_SIZE..].as_ptr() as *const minix3_super_block;
        let super_block = unsafe { core::ptr::read_unaligned(super_block_ptr) };

        // TODO 期待する値をイコールに変更したい
        assert!(
            super_block.s_ninodes > 0,
            "s_ninodes should be greater than 0"
        );
        assert!(
            super_block.s_imap_blocks > 0,
            "s_imap_blocks should be greater than 0"
        );
        assert!(
            super_block.s_zmap_blocks > 0,
            "s_zmap_blocks should be greater than 0"
        );
        assert!(
            super_block.s_firstdatazone > 0,
            "s_firstdatazone should be greater than 0"
        );
        assert!(super_block.s_zones > 0, "s_zones should be greater than 0");

        // inode, zoneのbitmapの[0]と[1]が1になっていることを確認する
        let imap_start_block = super_block.imap_start_block();
        let imap_byte_offset = imap_start_block * BLOCK_SIZE;
        let imap_byte = mem[imap_byte_offset];
        assert!(
            imap_byte == 0b00000011,
            "First two inodes should be marked as used in imap"
        );

        let zmap_start_block = super_block.zmap_start_block();
        let zmap_byte_offset = zmap_start_block * BLOCK_SIZE;
        let zmap_byte = mem[zmap_byte_offset];
        assert!(
            zmap_byte == 0b00000011,
            "First two zones should be marked as used in zmap"
        );

        // ルートディレクトリのinodeの初期化チェック
        let inode_table_block =
            2 + super_block.s_imap_blocks as usize + super_block.s_zmap_blocks as usize;
        let root_inode_offset = inode_table_block * BLOCK_SIZE;
        let root_inode_ptr = mem[root_inode_offset..].as_ptr() as *const minix3_inode;
        let root_inode = unsafe { core::ptr::read_unaligned(root_inode_ptr) };

        let i_mode = root_inode.i_mode;
        assert_eq!(i_mode & 0x4000, 0x4000, "Root inode should be a directory");

        let i_size = root_inode.i_size;
        assert_eq!(i_size, 128, "Root inode size should be 128");

        // ルートディレクトリのzone[0]に自身を指す. と .. が存在する
        let root_zone_offset = root_inode.i_zone[0] as usize * BLOCK_SIZE;
        let entries_count = BLOCK_SIZE / core::mem::size_of::<minix3_dir_entry>();
        let entries = unsafe {
            core::slice::from_raw_parts(
                mem.as_ptr().add(root_zone_offset) as *const minix3_dir_entry,
                entries_count,
            )
        };
        let mut found_dot = false;
        let mut found_dotdot = false;
        for entry in entries {
            let len = entry.name.iter().position(|&c| c == 0).unwrap_or(60);
            let entry_name = &entry.name[..len];
            if entry_name == b"." {
                found_dot = true;
            } else if entry_name == b".." {
                found_dotdot = true;
            }
        }
        assert!(found_dot, "Root directory should contain '.' entry");
        assert!(found_dotdot, "Root directory should contain '..' entry");
    }

    #[test_case]
    fn minix_integration_test() {
        const BLOCK_SIZE: usize = 1024;
        let fs = minix3_inode::new(0);
        let size = 4 * 1024 * 1024;
        let mut mem = alloc::vec![0u8; size];

        init_minixfs(&mut mem, BLOCK_SIZE);

        fs.mkdir(&mut mem, b"/nested_dir", BLOCK_SIZE);
        fs.mkdir(&mut mem, b"/nested_dir/nested_dir2", BLOCK_SIZE);
        fs.delete_dir_entry(&mut mem, b"/nested_dir/nested_dir2", BLOCK_SIZE);
        fs.mkdir(&mut mem, b"/nested_dir/nested_dir2", BLOCK_SIZE);
        fs.create_file(&mut mem, BLOCK_SIZE, b"/nested_dir/nested_dir2/test.txt");
        fs.write(
            &mut mem,
            b"/nested_dir/nested_dir2/test.txt",
            BLOCK_SIZE,
            b"filesystem integration test",
        );

        let result = fs
            .read(&mut mem, b"/nested_dir/nested_dir2/test.txt", BLOCK_SIZE)
            .unwrap();
        assert_eq!(result, b"filesystem integration test");
    }
}
