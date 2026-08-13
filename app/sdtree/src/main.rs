#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

use noli::error::Error;
use noli::prelude::*;

fn main() -> Result<()> {
    Api::write_string("show directory tree demo!\n");
    Api::show_directory_tree();
    Api::exit(42);
}

entry_point!(main);
