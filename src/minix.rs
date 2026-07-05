use crate::info;
use core::mem::size_of;

const MINIX_BLOCK_SIZE: usize = 1024;

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
    pub name: [char; 30],
}

pub fn read_magic(minix_img: &[u8]) -> u16 {
    // 引数にイメージのバイナリを取る
    // 第一ブロックを読み飛ばす
    let super_block = unsafe {
        *(minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };
    super_block.s_magic
}

pub fn read_file_name(minix_img: &[u8]) {
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

            // zone の[0] ~
            // [6]は直接ブロック番号が入っているので、
            // zone_bitmapと照らし合わせると良い?
            // とりあえずルート以下のファイル名を取得する、
            // ルートのzone[0]を読むと良さそう(明らかに1024バイトもない)
            let zone_block = inode.i_zone[0] as usize;
            let zone_offset = zone_block * (MINIX_BLOCK_SIZE as usize);

            //　一旦バイト列として読み出す

            let tmp = unsafe {
                *(minix_img.as_ptr().add(zone_offset).add(i * 8) as *const u8)
            };

            info!("{}", tmp as char);
        }
    }
}
