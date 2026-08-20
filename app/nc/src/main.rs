#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

extern crate alloc;

use alloc::string::String;
use core::str::FromStr;
use noli::args;
use noli::error::{Error, Result};
use noli::net::{lookup_host, IpV4Addr, SocketAddr, TcpStream};
use noli::prelude::*;

fn usage() {
    println!("Usage: nc <host> <port>");
}

fn parse_args<'a>(args: &'a [&'a str]) -> Result<(&'a str, u16)> {
    let [_, host, port] = args else {
        return Err(Error::Failed("Invalid arguments"));
    };
    let port = port
        .parse::<u16>()
        .map_err(|_| Error::Failed("Invalid port"))?;
    if port == 0 {
        return Err(Error::Failed("Port must not be zero"));
    }
    Ok((host, port))
}

fn resolve_host(host: &str) -> Result<IpV4Addr> {
    if let Ok(ip) = IpV4Addr::from_str(host) {
        return Ok(ip);
    }
    lookup_host(host)?
        .first()
        .copied()
        .ok_or(Error::Failed("Host has no IPv4 address"))
}

fn write_all(stream: &mut TcpStream, mut data: &[u8]) -> Result<()> {
    while !data.is_empty() {
        let written = stream.write(data)?;
        if written == 0 {
            return Err(Error::Failed("TCP write made no progress"));
        }
        data = &data[written..];
    }
    Ok(())
}

/// Reads one line from the keyboard. Ctrl-D ends the command.
fn read_line() -> Option<String> {
    let mut line = String::new();
    loop {
        if let Some(c) = Api::read_key() {
            if c == '\u{4}' {
                return None;
            }
            print!("{c}");
            line.push(c);
            if c == '\n' {
                return Some(line);
            }
        }
    }
}

fn run(host: &str, port: u16) -> Result<()> {
    let ip = resolve_host(host)?;
    let socket_addr: SocketAddr = (ip, port).into();
    let mut stream = TcpStream::connect(socket_addr)?;
    println!("Connected to {host} ({ip}) port {port}");
    println!("Type a line to send. Ctrl-D exits.");

    while let Some(line) = read_line() {
        write_all(&mut stream, line.as_bytes())?;

        let mut buf = [0u8; 4096];
        let bytes_read = stream.read(&mut buf)?;
        if bytes_read == 0 {
            println!("Connection closed by peer.");
            break;
        }
        print!("{}", String::from_utf8_lossy(&buf[..bytes_read]));
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = args::from_env();
    let (host, port) = match parse_args(&args) {
        Ok(values) => values,
        Err(error) => {
            usage();
            return Err(error);
        }
    };
    run(host, port)
}

entry_point!(main);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_and_port() {
        assert_eq!(
            parse_args(&["nc", "example.com", "8080"]).unwrap(),
            ("example.com", 8080)
        );
    }

    #[test]
    fn rejects_invalid_arguments() {
        assert!(parse_args(&["nc"]).is_err());
        assert!(parse_args(&["nc", "localhost", "0"]).is_err());
        assert!(parse_args(&["nc", "localhost", "invalid"]).is_err());
    }
}
