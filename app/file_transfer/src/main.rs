#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

extern crate alloc;

use alloc::{format, vec};
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
            let path = splitted_command.next();
            let size = splitted_command.next();
            match (path, size) {
                (Some(path), Some(size)) => {
                    let filesize: usize = size
                        .parse()
                        .map_err(|_| Error::Failed("Invalid file size"))?;
                    let mut data = vec![0; filesize];
                    read_exact(stream, &mut data)?;
                    handle_put(path, &data, stream)
                }
                _ => {
                    Api::write_string("path or size is empty in put request\n");
                    Ok(())
                }
            }
        }
        Some("dir") => handle_dir(stream),
        _ => {
            Api::write_string("**** invalid commnad\n");
            Ok(())
        }
    }
}

fn handle_dir(stream: &mut TcpStream) -> Result<()> {
    let mut entries = [0u8; 2048];
    let entries_len = Api::list_dir_entries(b"/", &mut entries);
    if entries_len < 0 {
        const RESPONSE: &[u8] = b"Err dir failed\n";
        if stream.write(RESPONSE)? != RESPONSE.len() {
            return Err(Error::Failed("Incomplete DIR error response write"));
        }
        return Ok(());
    }

    let entries_len = entries_len as usize;
    let mut response = format!("OK {}\n", entries_len).into_bytes();
    response.extend_from_slice(&entries[..entries_len]);

    if stream.write(&response)? != response.len() {
        return Err(Error::Failed("Incomplete DIR response write"));
    }
    Ok(())
}

fn read_exact(stream: &mut TcpStream, mut data: &mut [u8]) -> Result<()> {
    while !data.is_empty() {
        let bytes_read = stream.read(data)?;
        if bytes_read == 0 {
            return Err(Error::Failed("Connection closed during PUT"));
        }
        data = &mut data[bytes_read..];
    }
    Ok(())
}

fn read_command_line(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<usize> {
    let mut len = 0;
    loop {
        if len == buffer.len() {
            return Err(Error::Failed("Command line is too long"));
        }

        let bytes_read = stream.read(&mut buffer[len..len + 1])?;
        if bytes_read == 0 {
            return Ok(0);
        }
        if buffer[len] == b'\n' {
            return Ok(len);
        }
        len += 1;
    }
}

fn handle_get(path: &str, stream: &mut TcpStream) -> Result<()> {
    Api::write_string(" get request received\n");

    if path == "/proc/fs.img" {
        Api::write_string("proc/fs.img sending ... \n");

        let mut buf = [0u8; 512 * 64];
        let image_size = Api::fs_img(&mut buf);
        if image_size < 0 {
            if stream.write(b"-1\n")? != 3 {
                return Err(Error::Failed("Incomplete GET error response write"));
            }
            return Ok(());
        }

        let header = format!("{}\n", image_size);
        stream.write(header.as_bytes())?;

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
            let file_len = bytes_read as usize;
            let mut response = format!("{}\n", file_len).into_bytes();
            response.extend_from_slice(&buffer[..file_len]);

            let bytes_written = stream.write(&response)?;
            if bytes_written != response.len() {
                return Err(Error::Failed("Incomplete file write"));
            }
        } else {
            Api::write_string("**** read file failed\n");
            if stream.write(b"-1\n")? != 3 {
                return Err(Error::Failed("Incomplete GET error response write"));
            }
        }
        Api::write_string("send file done\n");
        Ok(())
    }
}

fn handle_put(path: &str, data: &[u8], stream: &mut TcpStream) -> Result<()> {
    Api::write_string("put request received\n");

    let (_, name) = split_path_and_filename(path.as_bytes());
    let succeeded =
        Api::create_file(name) >= 0 && Api::write_all_file(name, data) == data.len() as i64;

    let response: &[u8] = if succeeded { b"ok\n" } else { b"failed\n" };
    let bytes_written = stream.write(response)?;
    if bytes_written != response.len() {
        return Err(Error::Failed("Incomplete PUT response write"));
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
        let bytes_read = read_command_line(&mut stream, &mut buf)?;

        // 接続が閉じられたら次の接続を待つ
        if bytes_read == 0 {
            continue;
        }

        let command = str::from_utf8(&buf[..bytes_read])
            .map_err(|_| Error::Failed("Invalid UTF-8 sequence"))?;
        if command == "exit" {
            stream.write(b"bye\n")?;
            break;
        }
        handle_command(command, &mut stream)?;
    }

    Ok(())
}

entry_point!(main);
