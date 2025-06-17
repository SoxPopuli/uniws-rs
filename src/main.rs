use std::{fs::File, io::BufReader};

mod error;
mod config;

fn main() {
    let config = include_str!("../patches.ini");
    config::parse(config).unwrap();
}
