use ansibleops::validation::*;

#[test]
fn test_port_validation_boundary_values() {
    assert_eq!(validate_port("1").unwrap(), 1);
    assert_eq!(validate_port("65535").unwrap(), 65535);
    assert!(validate_port("0").is_err());
    assert!(validate_port("65536").is_err());
    assert!(validate_port("").is_err());
    assert!(validate_port("abc").is_err());
    assert!(validate_port("-1").is_err());
    assert!(validate_port("  80  ").unwrap() == 80);
}

#[test]
fn test_host_validation() {
    validate_host("web1.example.com").unwrap();
    validate_host("192.168.1.1").unwrap();
    validate_host("my-host_01").unwrap();
    validate_host("localhost").unwrap();
    assert!(validate_host("").is_err());
    assert!(validate_host("host with spaces").is_err());
    assert!(validate_host("host;rm -rf /").is_err());
    assert!(validate_host("host$(whoami)").is_err());
    assert!(validate_host("host`id`").is_err());
}

#[test]
fn test_extra_vars_validation() {
    validate_extra_vars("").unwrap();
    validate_extra_vars("key: value").unwrap();
    validate_extra_vars("foo: bar\nbaz: 123").unwrap();
    assert!(validate_extra_vars(": invalid yaml: [").is_err());
}

#[test]
fn test_hosts_validation() {
    validate_hosts("web1,web2,db1").unwrap();
    validate_hosts("all").unwrap();
    assert!(validate_hosts("").is_err());
    assert!(validate_hosts("bad host;").is_err());
    assert!(validate_hosts("web1, bad host").is_err());
}

#[test]
fn test_file_path_validation() {
    use std::path::Path;
    assert!(validate_file_path("", true).is_err());
    assert!(validate_file_path("/nonexistent/file.txt", true).is_err());
    assert!(validate_file_path("Cargo.toml", true).is_ok());
    assert!(validate_file_path("/tmp", false).is_ok());
}

#[test]
fn test_lang_normalization() {
    assert_eq!(normalize_lang("ru"), Lang::Ru);
    assert_eq!(normalize_lang("Русский"), Lang::Ru);
    assert_eq!(normalize_lang("russian"), Lang::Ru);
    assert_eq!(normalize_lang("en"), Lang::En);
    assert_eq!(normalize_lang("english"), Lang::En);
    assert_eq!(normalize_lang(""), Lang::En);
}
