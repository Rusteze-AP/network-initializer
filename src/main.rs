extern crate network_initializer as lib;

use lib::Config;

fn main() {
    let path = "initialization_files/test.toml";
    let mut config = Config::new(Some(path));
    assert!(config.is_ok(), "{}", config.err().unwrap());
    let config = config.unwrap();
    println!("{config:#?}");
}
