#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

use noli::net::TcpStream;
use noli::prelude::*;
use core::str;
use noli::error::Error;
use Api;



fn handle_command(command: &str , stream: & mut TcpStream) -> Result<()> {
    
    Api::write_string(command);
    let mut splitted_command = command.split_whitespace();

   match splitted_command.next() {
        Some("get") => {
            let path = splitted_command.next();
            match path {
                Some(path) =>   handle_get(path , stream) , 
                None => {      
                    Api::write_string("**** empty path\n");
                   Ok(())
                },
            }
        } , 
        _ => {
            Api::write_string("**** invalid commnad\n");
            Ok(())
        }
    }
}

fn handle_get(path: &str, stream: &mut TcpStream) -> Result<()> {
   
    Api::write_string(" get request received\n");

    let mut buffer = [0u8; 1024];
    let bytes_read = Api::read_all_file(path.as_bytes(), &mut buffer);

    if bytes_read > 0 {
        let mut message = [0u8; 21];

        let mut value = bytes_read as u64;
        let mut reversed_digits = [0u8; 20];
        let mut digit_count = 0;
        while value > 0 {
            reversed_digits[digit_count] = b'0' + (value % 10) as u8;
            digit_count += 1;
            value /= 10;
        }
        for i in 0..digit_count {
            message[i] = reversed_digits[digit_count - i - 1];
        }
        message[digit_count] = b'\n';

        stream.write(&message[..digit_count + 1])?;
        stream.write(&buffer[..bytes_read as usize])?;
    } else {
        Api::write_string("**** read file faile\n");
    }
    Ok(())
}

fn main() -> Result<()> {
    Api::write_string("**** Hello from a test ftp modoki server!\n");

    let mut stream = TcpStream::open_easy_tcp_server(18083)?;

    Api::write_string("**** TCP connection established!\n");

    // stream.write(b"\n")?;
    let mut buf = [0u8; 1024];
    loop {
        Api::write_string("**** listening request in port 18083...\n");
        let bytes_read = stream.read(&mut buf)?;

        // 接続が閉じられたらbreak
        if bytes_read == 0 {
            break;
        }

        match str::from_utf8(&buf[..bytes_read]) {
            Ok(command) => handle_command(command , & mut stream)?,
            Err(_) => return Err(Error::Failed("Invalid UTF-8 sequence")),
        }
    }

    Api::exit(42);
}

entry_point!(main);
