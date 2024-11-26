mod parser {
    use network_initializer::errors::ConfigError;
    use network_initializer::NetworkInitializer;

    #[test]
    fn test_ok() {
        let path = "initialization_files/test_files/bidirectionality/ok.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_ok(), "{}", config.err().unwrap());
    }

    #[test]
    fn test_drone_err() {
        let path = "initialization_files/test_files/bidirectionality/drone_err.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_err());
        assert_eq!(
            config.err().unwrap(),
            ConfigError::UnidirectionalConnection(1, 2)
        );
    }

    #[test]
    fn test_client_err() {
        let path = "initialization_files/test_files/bidirectionality/client_err.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_err());
        assert_eq!(
            config.err().unwrap(),
            ConfigError::UnidirectionalConnection(2, 5)
        );
    }

    #[test]
    fn test_server_err() {
        let path = "initialization_files/test_files/bidirectionality/server_err.toml";
        let config = NetworkInitializer::new(Some(path));

        assert!(config.is_err());
        assert_eq!(
            config.err().unwrap(),
            ConfigError::UnidirectionalConnection(6, 1)
        );
    }
}
