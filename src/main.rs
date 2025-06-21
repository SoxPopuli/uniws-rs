mod config;
mod signature;
mod error;
mod patch_info;

fn main() {
    let config = include_str!("../patches.ini");
    let config = config::Config::new(config).unwrap();

    let app = &config.sections[0];

    for p in &app.patches {
        p.apply_patch(&mut [], 1, 2);
    }


    println!("{config:#?}");
}
