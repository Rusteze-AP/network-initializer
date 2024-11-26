mod parser {
    use network_initializer::errors::ConfigError;
    use network_initializer::NetworkInitializer;

    #[test]
    fn test_ok() {
        let path = "initialization_files/test_files/unique_ids/ok.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_ok(), "{}", config.err().unwrap());
    }

    #[test]
    fn test_duplicate_ids() {
        let path = "initialization_files/test_files/unique_ids/duplicate_ids.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_err());
        assert_eq!(config.err().unwrap(), ConfigError::DuplicatedNodeId);
    }
}
