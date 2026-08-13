#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

use core::str;
use noli::error::Error;
use noli::net::TcpStream;
use noli::prelude::*;
use Api;

fn split_path_and_filename(file_path: &[u8]) -> (&[u8], &[u8]) {
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

fn handle_command(command: &str, stream: &mut TcpStream) -> Result<()> {
    Api::write_string(command);
    let mut splitted_command = command.split_whitespace();

    match splitted_command.next() {
        Some("get") => {
            let path = splitted_command.next();
            match path {
                Some(path) => handle_get(path, stream),
                None => {
                    Api::write_string("**** empty path\n");
                    Ok(())
                }
            }
        }
        Some("put") => {
            // TODO : splitted_comand.next(); に失敗したときの処理を詰める
            let path = splitted_command.next();
            let size = splitted_command.next();
            let data = splitted_command.next();
            match (path, size , data) {
                (Some(path), Some(size) , Some(data)) => {
                    let filesize: u32 = size
                        .parse()
                        .map_err(|_| Error::Failed("Invalid file size"))?;
                    let data: &[u8] = data.as_bytes();
                    handle_put(path, filesize, data, stream)
                }
                _ => {
                    Api::write_string("path or size or data is empty in put request\n");
                    Ok(())
                }
            }
        }
        _ => {
            Api::write_string("**** invalid commnad\n");
            Ok(())
        }
    }
}

fn handle_get(path: &str, stream: &mut TcpStream) -> Result<()> {
    Api::write_string(" get request received\n");

    if path == "/proc/fs.img" {
        Api::write_string("proc/fs.img sending ... \n");

        let mut buf = [0u8; 512 * 64];
        let image_size = Api::fs_img(&mut buf);
        if image_size < 0 {
            return Err(Error::Failed("Failed to read MinixFS image"));
        }

        let mut message = [0u8; 21];
        let mut value = image_size as u64;

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

        for chunk in buf[..image_size as usize].chunks(1024) {
            let bytes_written = stream.write(chunk)?;

            if bytes_written != chunk.len() {
                return Err(Error::Failed("Incomplete MinixFS image write"));
            }
            Api::write_string("write chunk done\n");
        }
           Api::write_string("send /proc/fs.img done\n");
        Ok(())
    } else {
        Api::write_string("file sending ... \n");
        let mut buffer = [0u8; 2048];
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

            let header_len = digit_count + 1;
            let file_len = bytes_read as usize;
            let mut response = [0u8; 21 + 2048];
            response[..header_len].copy_from_slice(&message[..header_len]);
            response[header_len..header_len + file_len].copy_from_slice(&buffer[..file_len]);

            let response_len = header_len + file_len;
            let bytes_written = stream.write(&response[..response_len])?;
            if bytes_written != response_len {
                return Err(Error::Failed("Incomplete file write"));
            }
        } else {
            Api::write_string("**** read file failed\n");
        }
          Api::write_string("send file done\n");
        Ok(())
    }
}

fn handle_put(path: &str, filesize: u32,  data : &[u8] , stream: &mut TcpStream) -> Result<()> {
    Api::write_string("put request received\n");

     //  1.pathをsplitする
     let (parent , name ) = split_path_and_filename(path.as_bytes());
     // 2. ファイルを作成する
     Api::create_file(name);
     // 3. データを書き込む
     Api::write_all_file(name ,  data);

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

        // 接続が閉じられたら次の接続を待つ
        if bytes_read == 0 {
            continue;
        }

        match str::from_utf8(&buf[..bytes_read]) {
            Ok(command) => handle_command(command, &mut stream)?,
            Err(_) => return Err(Error::Failed("Invalid UTF-8 sequence")),
        }
    }
}

entry_point!(main);
