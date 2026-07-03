const  MINIX_BLOCK_SIZE: usize =  1024;

#[repr(C , packed)]
#[derive(Clone, Copy)]
pub  struct minix_super_block {
    pub s_ninodes : u16,
    pub s_nzones: u16,
    pub s_imap_blocks :u16,
    pub s_zmap_blocks :u16,
    pub s_firstdatazone : u16 , 
    pub s_log_zone_size: u16 , 
    pub s_max_size: u32 , 
    pub s_magic : u16 , 
    pub s_state: u16 , 
    pub s_zones: u16,
}

#[repr(C , packed)]
pub struct minix_inode {
    pub i_mode: u16 , 
    pub i_uid : u16 , 
    pub i_size : u32 , 
    pub i_time : u32 ,
    pub i_gid : u8, 
    pub i_nlinks : u8, 
   pub i_zone: [u16; 9],
}

#[repr(C , packed)]
pub struct minix_dir_entry {
    pub inode : u16 , 
    // char name[]; でこまった
    pub name : [char ; 14],
}


pub fn  read_magic (minix_img : &[u8]) -> u16{
    // 引数にイメージのバイナリを取る
    // 第一ブロックを読み飛ばす

    let super_block = unsafe{
        * (minix_img.as_ptr().add(MINIX_BLOCK_SIZE) as *const minix_super_block)
    };
     super_block.s_magic
}

pub fn interpretation_minix_img (minix_img :&[u8]){
    // イメージのバイナリを渡される
}








