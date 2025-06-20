mod config;
mod signature;
mod error;
mod patch_info;

fn main() {
    let config = include_str!("../patches.ini");
    let config = config::Config::new(config).unwrap();

    println!("{config:#?}");
}
