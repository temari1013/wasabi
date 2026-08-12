#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

use noli::error::Error;
use noli::prelude::*;

fn main() -> Result<()> {
    Api::write_string("show directory tree demo!\n");
    Api::show_directory_tree();

<<<<<<< HEAD
      let mut buffer = [0u8; 1024];
      let bytes_read = Api::read_all_file(b"/test.txt", &mut buffer);

      if bytes_read < 0 {
          return Err(Error::Failed("read_all_file failed"));
      }

      let data = &buffer[..bytes_read as usize];
      println!("read_all_file: {} bytes: {:?}", bytes_read, data);
=======
    let mut buffer = [0u8; 1024];
    let bytes_read = Api::read_direct_file(b"/test.txt", &mut buffer);

    if bytes_read < 0 {
        return Err(Error::Failed("read_direct_file failed"));
    }

    let data = &buffer[..bytes_read as usize];
    println!("read_direct_file: {} bytes: {:?}", bytes_read, data);
>>>>>>> 00f29e5dca480278b540e7178ea234fd6ee5fc4d

    Api::exit(42);
}

entry_point!(main);
