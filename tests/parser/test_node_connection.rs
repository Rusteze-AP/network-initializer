mod parser {
    use network_initializer::errors::ConfigError;
    use network_initializer::NetworkInitializer;

    #[test]
    fn test_ok() {
        let path = "initialization_files/test_files/node_connection/ok.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_ok(), "{}", config.err().unwrap());
    }

    #[test]
    fn test_empty_topology_err() {
        let path = "initialization_files/test_files/node_connection/err_node_connection.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_err());
        assert_eq!(
            config.err().unwrap(),
            ConfigError::InvalidNodeConnection(6, 9)
        );
    }
}
