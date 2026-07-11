use crate::error;
use crate::info;
use core::{mem::size_of, panic::PanicInfo, str::Chars};

const MINIX_BLOCK_SIZE: usize = 1024;
const I_DIRECTORY: u16 = 0040000;
pub type c_char = i8;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct minix_super_block {
    pub s_ninodes: u16,
    pub s_nzones: u16,
    pub s_imap_blocks: u16,
    pub s_zmap_blocks: u16,
    pub s_firstdatazone: u16,
    pub s_log_zone_size: u16,
    pub s_max_size: u32,
    pub s_magic: u16,
    pub s_state: u16,
    pub s_zones: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct minix_inode {
    pub i_mode: u16,
    pub i_uid: u16,
    pub i_size: u32,
    pub i_time: u32,
    pub i_gid: u8,
    pub i_nlinks: u8,
    pub i_zone: [u16; 9],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct minix_dir_entry {
    pub inode: u16,
    // char name[]; でこまった
    pub name: [c_char; 30],
}

pub fn read_magic(minix_img: &[u8]) -> u16 {
    // 引数にイメージのバイナリを取る
    // 第一ブロックは起動なので読み飛ばす
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };
    super_block.s_magic
}

pub fn get_inode(inode_num: u16, minix_img: &[u8]) -> *mut minix_inode {
    // inodeの開始位置ブロックが必要
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };
    // super_block
    let inodes_offset = 2
        + super_block.s_zmap_blocks as usize
        + super_block.s_imap_blocks as usize;
    let tmp = unsafe {
        minix_img
            .as_ptr()
            .add(MINIX_BLOCK_SIZE * inodes_offset)
            .add((inode_num - 1) as usize * 32) as *mut minix_inode
    };
    tmp
}

pub fn read_file_zone(inode: *const minix_inode, minix_img: &[u8]) {
    let zone_0 = unsafe { (*inode).i_zone[0] as usize };
    let file_size = unsafe { (*inode).i_size as usize };

    if zone_0 == 0 || file_size == 0 {
        return;
    }

    let zone_offset = zone_0 * MINIX_BLOCK_SIZE;

    let read_len = core::cmp::min(file_size, MINIX_BLOCK_SIZE);

    let file_data = unsafe {
        core::slice::from_raw_parts(
            minix_img.as_ptr().add(zone_offset),
            read_len,
        )
    };

    let text = core::str::from_utf8(file_data).unwrap_or("<invalid utf8>");
    info!("file_data:");
    info!("{}", text);
}

pub fn read_all_directoies(root_inode: *const minix_inode, minix_img: &[u8]) {
    info!("directory:");
    read_file_name(root_inode, minix_img);
}

// 全ファイルのファイル名とデータ部を再帰的に取得する
pub fn read_file_name(root_inode: *const minix_inode, minix_img: &[u8]) {
    // 　渡されたinodeの子を再帰的にたどる

    let zone_block = unsafe { (*root_inode).i_zone[0] as usize };
    if zone_block == 0 {
        return;
    }
    let zone_offset = zone_block * (MINIX_BLOCK_SIZE as usize);
    let file_size = unsafe { (*root_inode).i_size as usize };
let num_entries = core::cmp::min(file_size / 32, 32);

    for i in 0..num_entries {
        let tmp = unsafe {
            *(minix_img.as_ptr().add(zone_offset).add(i * 32)
                as *const minix_dir_entry)
        };
        if tmp.inode == 0 {
            continue;
        }

        let name_len = tmp.name.iter().position(|&c| c == 0).unwrap_or(30);
        let utf8_bytes = unsafe {
            core::slice::from_raw_parts(
                tmp.name.as_ptr() as *const u8,
                name_len,
            )
        };

        let file_name =
            core::str::from_utf8(utf8_bytes).unwrap_or("<invalid utf8>");
        info!("{}", file_name);

        // tmp.inode は inode番号を返す
        // カレントディレクトリと未使用inodeを弾く
        if tmp.name[0] != b'.' as i8 {
            let is_used = (minix_img[1024 * 2 + (tmp.inode as usize / 8)]
                & (1 << (tmp.inode as usize % 8)))
                != 0;
            let inode = get_inode(tmp.inode, minix_img);
            if is_used {
                let i_mode = unsafe { (*inode).i_mode };
                if (i_mode & 0o170000) == 0o040000 {
                    read_file_name(inode, minix_img);
                } else {
                    read_file_zone(inode, minix_img);
                }
            }
        }
    }
}

pub fn read_all_inode(minix_img: &[u8]) {
    // イメージのバイナリを渡される
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };
    // super_block.
    let inodes_num = super_block.s_ninodes as usize;
    let znodes_blocks = super_block.s_zmap_blocks as usize;
    let inodes_blocks = super_block.s_imap_blocks as usize;

    // 全inodesについて
    for i in 0..inodes_num {
        // 1.ビットマップを見て、使われているかどうかを判断する
        // 開始ビットは + 2048?
        let is_used = (minix_img[1024 * 2 + (i / 8)] & (1 << (i % 8))) != 0;
        if is_used {
            // inodeが使われているのでnodeを読み取る処理が必要
            // inodeの最初のブロックのオフセットブロック数は、2 + znodes_blocks
            // + inodes_blocks ブロック内でのオフセットは I *
            // size_of(inode)で取得できる
            let block_offset =
                2 + (znodes_blocks as usize) + (inodes_blocks as usize);
            let inner_block_offset = (i as usize) * size_of::<minix_inode>();

            //inode
            let inode = unsafe {
                *(minix_img
                    .as_ptr()
                    .add((MINIX_BLOCK_SIZE as usize) * block_offset)
                    .add(inner_block_offset)
                    as *const minix_inode)
            };

            // zone の[0]
            // [6]は直接ブロック番号が入っているので、
            // zone_bitmapと照らし合わせると良い?
            // とりあえずルート以下のファイル名を取得する、
            // ルートのzone[0]を読むと良さそう(明らかに1024バイトもない)
            let zone_block = inode.i_zone[0] as usize;
            let zone_offset = zone_block * (MINIX_BLOCK_SIZE as usize);

            //　一旦バイト列として読み出す。
            // この中にはファイルの名前とinodeが格納されているはず
            
            for i in 0..32 {
                let tmp = unsafe {
                    // as const minix_dir_entry失敗したらどうなるのか調査が必要
                    *(minix_img.as_ptr().add(zone_offset).add(i * 32)
                        as *const minix_dir_entry)
                };
                for j in 0..30 {
                    let c = tmp.name[j] as u8 as char;
                    if c != '\0' {
                        info!("{}", c);
                    }
                }
            }
        }
    }
}

pub fn alloc_inode(minix_img: &mut [u8]) -> usize {
    // 現時点ではinodeはファイル作成にしか使わない
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };

    let max_bits = (super_block.s_imap_blocks as usize) * MINIX_BLOCK_SIZE * 8;
    let mut inode_num = 0;

    // 使える最初のinodeを見つける
    for i in 1..max_bits {
        let byte_idx = MINIX_BLOCK_SIZE * 2 + (i / 8);
        let bit_idx = i % 8;
        let is_used = (minix_img[byte_idx] & (1u8 << bit_idx)) != 0;
        if !is_used && i != 0 {
            inode_num = i;
            minix_img[byte_idx] |= 1u8 << bit_idx;
            break;
        }
    }

    // 使えるinode番号にinodeを書き込む

    let inode_table_block = 2
        + (super_block.s_imap_blocks as usize)
        + (super_block.s_zmap_blocks as usize);
    let inode_offset = (inode_table_block * MINIX_BLOCK_SIZE)
        + (inode_num - 1) * core::mem::size_of::<minix_inode>();

    // zone のためにアロックするブロックは、2 + bitmap二種 + inodeのブロック数 +
    // zone_num

    let node = minix_inode {
        i_mode: 0,
        i_uid: 0,
        i_size: 0,
        i_time: 0,
        i_gid: 0,
        i_nlinks: 0,
        i_zone: [0; 9],
    };
    unsafe {
        let target_ptr =
            minix_img.as_mut_ptr().add(inode_offset) as *mut minix_inode;
        core::ptr::write(target_ptr, node);
    }
    inode_num
}
// createは空いてるところにアロックすればok

pub fn create_file(minix_img: &mut [u8], file_path: &[u8]) -> *mut minix_inode {
    // inodeと同じ要領で空いているzoneを探す
    // zone はブロック単位で割り当てるのが差異
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };

    let inode_num = alloc_inode(minix_img);
    if inode_num == 0 {
        error!("alloc inode failed")
    }

    let last_slash_idx = file_path.iter().rposition(|&b| b == b'/');

    let (parent_path, file_name) = match last_slash_idx {
        Some(idx) => {
            let parent = if idx == 0 {
                b"/".as_slice()
            } else {
                &file_path[..idx]
            };
            let name = &file_path[idx + 1..];
            (parent, name)
        }
        None => (b"/".as_slice(), file_path),
    };

    let parent_inode = inode_by_path(minix_img, parent_path);
    link_inode(minix_img, parent_inode, inode_num as u16, file_name);
    // link_inode を呼び出して作成したinodeを書き込む必要がある
    // それって本来的にはファイルのパス解決が必要なのでは?
    // => create_file は, inodeをアロックする処理であるべき。

    let ret = get_inode(inode_num as u16, minix_img);
    ret
}

pub fn inode_by_path(minix_img: &mut [u8], file_path: &[u8]) -> u16 {
    // ファイルパスを分割する
    // 分割したパス事にinodeを取得し、
    // そのinodeを親に検索して末尾までたどりつけると良い
    let components = file_path
        .split(|&b| b == b'/')
        .filter(|comp| !comp.is_empty());

    let mut current_inode_num = 1;

    for comp in components {
        let current_inode_ptr = get_inode(current_inode_num, minix_img);

        let zone_block = unsafe { (*current_inode_ptr).i_zone[0] as usize };
        let zone_offset = zone_block * MINIX_BLOCK_SIZE;
        let mut found = false;

        for i in 0..32 {
            let tmp = unsafe {
                *(minix_img.as_ptr().add(zone_offset).add(i * 32)
                    as *const minix_dir_entry)
            };
            if tmp.inode == 0 {
                continue;
            }
            let name_len = tmp.name.iter().position(|&c| c == 0).unwrap_or(30);
            let utf8_bytes = unsafe {
                core::slice::from_raw_parts(
                    tmp.name.as_ptr() as *const u8,
                    name_len,
                )
            };
            if utf8_bytes == comp {
                current_inode_num = tmp.inode;
                found = true;
                break;
            }
        }
        if !found {
            error!("can't find inode_by_path");
        }
    }
    current_inode_num
}

pub fn link_inode(
    minix_img: &mut [u8],
    parent_inode_num: u16,
    new_inode_num: u16,
    file_name: &[u8],
) -> bool {
    let parent_inode_ptr = get_inode(parent_inode_num, minix_img);
    if parent_inode_ptr.is_null() {
        return false;
    }
  let parent_inode = unsafe { &mut *parent_inode_ptr };
    let zone0_offset = parent_inode.i_zone[0] as usize * MINIX_BLOCK_SIZE;

    let entries = unsafe {
        core::slice::from_raw_parts_mut(
            minix_img.as_mut_ptr().add(zone0_offset) as *mut minix_dir_entry,
            32,
        )
    };

    for entry in entries.iter_mut() {
        if entry.inode == 0 {
            entry.inode = new_inode_num;
            entry.name.fill(0);

            let copy_len = core::cmp::min(file_name.len(), 30);
            for i in 0..copy_len {
                entry.name[i] = file_name[i] as i8;
            }
            return true;
        }
    }
    false
}

pub fn alloc_zone(minix_img: &mut [u8], inode_num: u16) -> u16 {
    let inode = get_inode(inode_num, minix_img);
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };

    let block_offset = 2 + super_block.s_imap_blocks;
    let max_bits = (super_block.s_zmap_blocks as usize) * MINIX_BLOCK_SIZE * 8;

    for i in (super_block.s_firstdatazone as usize)..max_bits {
        let byte_idx = (block_offset as usize * MINIX_BLOCK_SIZE) + (i / 8);
        let bit_idx = i % 8;
        let is_used = (minix_img[byte_idx] & (1u8 << bit_idx)) != 0;

        let mut ret = 0;
        if !is_used {
            // ゾーンマップを書き換える
            minix_img[byte_idx] |= 1u8 << bit_idx;
            // inode.zone[0] に代入
            unsafe {
                (*inode).i_zone[0] = i as u16;
            }
            ret = i;
            return ret as u16;
        }
    }
    0 as u16
}

pub fn write_file(minix_img: &mut [u8], file_path: &[u8], data: &[u8]) {
    let inode_num = inode_by_path(minix_img, file_path);
    let inode = get_inode(inode_num, minix_img);
    let mut zone = unsafe { (*inode).i_zone[0] };

    if (zone == 0) {
        zone = alloc_zone(minix_img, inode_num);
    }

    let zone_byte = zone as usize * MINIX_BLOCK_SIZE;

    let copy_len = data.len();
    unsafe {
        let dst_ptr = minix_img.as_mut_ptr().add(zone_byte);
        for i in 0..copy_len {
            core::ptr::write(dst_ptr.add(i), data[i]);
        }
    }
}

// ディレクトリエントリを用意する
pub fn new_dir_entry(inode_num: u16, file_name: &[u8]) -> minix_dir_entry {
    let mut name = [0; 30];
    let copy_len = file_name.len();

    for i in 0..copy_len {
        name[i] = file_name[i] as i8;
    }

    minix_dir_entry {
        inode: inode_num,
        name,
    }
}

pub fn create_dir(minix_img: &mut [u8], dir_path: &[u8]) -> bool {
    let last_slash_idx = dir_path.iter().rposition(|&b| b == b'/');

    let (parent_path, file_name) = match last_slash_idx {
        Some(idx) => {
            let parent = if idx == 0 {
                b"/".as_slice()
            } else {
                &dir_path[..idx]
            };
            let name = &dir_path[idx + 1..];
            (parent, name)
        }
        None => {
            return false;
        }
    };

    let parent_inode_num = inode_by_path(minix_img, parent_path);
    if parent_inode_num == 0 {
        return false;
    }

    let new_inode_num = alloc_inode(minix_img) as u16;
    if new_inode_num == 0 {
        return false;
    }

    let new_zone_num = alloc_zone(minix_img, new_inode_num);
    if new_zone_num == 0 {
        return false;
    }

    let new_inode_ptr = get_inode(new_inode_num, minix_img);
    unsafe {
        // mode 多分違う
        (*new_inode_ptr).i_mode = 0o4444;
        (*new_inode_ptr).i_size = 64;
        (*new_inode_ptr).i_nlinks = 2;
        (*new_inode_ptr).i_zone[0] = new_zone_num as u16;
    }

    let zone_offset = new_zone_num as usize * MINIX_BLOCK_SIZE;

    // zoneにdir_entryの情報を格納する
    let dot_entry = new_dir_entry(new_inode_num, b".");
    let dotdot_entry = new_dir_entry(parent_inode_num, b"..");
    let empty_entry = new_dir_entry(0, b"");

    unsafe {
        let entries_ptr =
            minix_img.as_mut_ptr().add(zone_offset) as *mut minix_dir_entry;

        core::ptr::write(entries_ptr.add(0), dot_entry);
        core::ptr::write(entries_ptr.add(1), dotdot_entry);

        for i in 2..32 {
            core::ptr::write(entries_ptr.add(i), empty_entry);
        }
    }

    link_inode(minix_img, parent_inode_num, new_inode_num, file_name)
}