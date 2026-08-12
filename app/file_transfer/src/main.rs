#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

use noli::net::TcpStream;
use noli::prelude::*;
use core::str;

fn handle_get() -> Result<()> {
    Ok(())
}

fn main() -> Result<()> {
    Api::write_string("**** Hello from a test ftp modoki server!\n");

    let mut stream = TcpStream::open_easy_tcp_server(18083)?;
    Api::write_string("**** TCP connection established!\n");

    // stream.write(b"\n")?;
    let mut buf = [0u8; 1024];

    loop {
        let bytes_read = stream.read(&mut buf)?;

        // 接続が閉じられたらbreak
        if bytes_read == 0 {
            break;
        }

        match str::from_utf8(&buf[..bytes_read]) {
            Ok(command) => println!("command: {command}"),
            Err(_) => println!("UTF-8ではないデータを受信しました"),
        }
    }

    Api::exit(42);
}

entry_point!(main);
