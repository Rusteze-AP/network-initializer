use network_initializer::NetworkInitializer;

fn main() {
    let path = "initialization_files/test.toml";
    let config = NetworkInitializer::new(Some(path));
    assert!(config.is_ok(), "{}", config.err().unwrap());
    let mut config = config.unwrap();
    // println!("{config:#?}");
    config.run_simulation();
}
