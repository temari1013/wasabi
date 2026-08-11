#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

use noli::net::TcpStream;
use noli::prelude::*;

fn main() -> Result<()> {
    Api::write_string("**** Hello from a test tcp server!\n");

    let mut stream = Api::show_directory_tree;
    Api::write_string("**** TCP connection established!\n");
    stream.write(b"Hello from Wasabi TCP server!\n")?;

    Api::exit(42);
}

entry_point!(main);
