mod error;
mod config;

fn main() {
    let config = include_str!("../patches.ini");
    // let config = config::parse(config).unwrap();
    let config = config::Config::new(config).unwrap();

    println!("{config:#?}");
}
