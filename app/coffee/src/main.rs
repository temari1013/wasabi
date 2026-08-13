#![no_std]
#![cfg_attr(not(target_os = "linux"), no_main)]

extern crate alloc;

use noli::net::lookup_host;
use noli::prelude::*;

const QUERY_HOST: &str = "y1-coffee-kudasai-from-temari.invalid";
const QUERY_COUNT: usize = 10_000;

fn main() -> Result<()> {
    for query_number in 1..=QUERY_COUNT {
        println!("DNS query {query_number}/{QUERY_COUNT} for {QUERY_HOST}");
        match lookup_host(QUERY_HOST) {
            Ok(results) => println!("  {results:?}"),
            Err(error) => println!("  {error:?}"),
        }
    }
    Ok(())
}

entry_point!(main);
