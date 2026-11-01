use ansibleops::ansible::webapi::{generate_token, extract_bearer_token};

#[test]
fn test_token_is_48_hex_chars() {
    let token = generate_token();
    assert_eq!(token.len(), 48);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_token_is_unique() {
    let t1 = generate_token();
    let t2 = generate_token();
    assert_ne!(t1, t2);
}

#[test]
fn test_extract_bearer_token_standard() {
    let req = "GET /api/history HTTP/1.1\r\nAuthorization: Bearer abc123\r\nHost: localhost\r\n";
    assert_eq!(extract_bearer_token(req), Some("abc123"));
}

#[test]
fn test_extract_bearer_token_case_insensitive() {
    let req = "GET /api/history HTTP/1.1\r\nauthorization: bearer ABC123\r\nHost: localhost\r\n";
    assert_eq!(extract_bearer_token(req), Some("ABC123"));
}

#[test]
fn test_extract_bearer_token_missing_header() {
    let req = "GET /api/history HTTP/1.1\r\nHost: localhost\r\n";
    assert_eq!(extract_bearer_token(req), None);
}

#[test]
fn test_extract_bearer_token_wrong_scheme() {
    let req = "GET /api/history HTTP/1.1\r\nAuthorization: Basic abc123\r\nHost: localhost\r\n";
    assert_eq!(extract_bearer_token(req), None);
}

#[test]
fn test_extract_bearer_token_empty_value() {
    let req = "GET /api/history HTTP/1.1\r\nAuthorization: Bearer \r\nHost: localhost\r\n";
    assert_eq!(extract_bearer_token(req), Some(""));
}

#[test]
fn test_extract_bearer_token_multiple_headers() {
    let req = "GET /api/history HTTP/1.1\r\nAuthorization: Bearer token123\r\nContent-Type: application/json\r\n";
    assert_eq!(extract_bearer_token(req), Some("token123"));
}
