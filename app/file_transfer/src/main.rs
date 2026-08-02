#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

extern crate alloc;

use noli::prelude::*;

fn main() {
    print!("file_transfer.");
}

entry_point!(main);
