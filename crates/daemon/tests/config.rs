use crabtalk_daemon::config::{DEFAULT_CONFIG, DaemonConfig};

#[test]
fn parse_default_config_template() {
    DaemonConfig::from_toml(DEFAULT_CONFIG).expect("default config template should parse");
}
