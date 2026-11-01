use crate::models::Lang;

/// Ошибка валидации ввода пользователя.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    Empty(String),
    InvalidPath(String),
    InvalidPort(String),
    InvalidHost(String),
    InvalidYaml(String),
    TooLong { field: String, max: usize },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Empty(field) => write!(f, "Поле не заполнено: {field}"),
            ValidationError::InvalidPath(path) => write!(f, "Неверный путь: {path}"),
            ValidationError::InvalidPort(port) => write!(f, "Неверный порт (1–65535): {port}"),
            ValidationError::InvalidHost(host) => write!(f, "Неверный хост: {host}"),
            ValidationError::InvalidYaml(msg) => write!(f, "Неверный YAML: {msg}"),
            ValidationError::TooLong { field, max } => {
                write!(f, "Поле '{field}' слишком длинное (макс. {max} символов)")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Валидирует путь к файлу: не пустой, существует (если check_exists = true).
pub fn validate_file_path(path: &str, check_exists: bool) -> Result<(), ValidationError> {
    if path.trim().is_empty() {
        return Err(ValidationError::Empty("путь к файлу".into()));
    }
    let p = std::path::Path::new(path);
    if check_exists && !p.exists() {
        return Err(ValidationError::InvalidPath(path.into()));
    }
    Ok(())
}

/// Валидирует путь к каталогу: не пустой, существует.
pub fn validate_dir_path(path: &str) -> Result<(), ValidationError> {
    if path.trim().is_empty() {
        return Err(ValidationError::Empty("путь к каталогу".into()));
    }
    let p = std::path::Path::new(path);
    if !p.exists() || !p.is_dir() {
        return Err(ValidationError::InvalidPath(path.into()));
    }
    Ok(())
}

/// Валидирует TCP-порт (1–65535).
pub fn validate_port(port: &str) -> Result<u16, ValidationError> {
    let port = port.trim();
    if port.is_empty() {
        return Err(ValidationError::Empty("порт".into()));
    }
    let n: u16 = port.parse().map_err(|_| ValidationError::InvalidPort(port.into()))?;
    if n == 0 || n > 65535 {
        return Err(ValidationError::InvalidPort(port.into()));
    }
    Ok(n)
}

/// Валидирует имя хоста/адрес (базовая проверка).
pub fn validate_host(host: &str) -> Result<(), ValidationError> {
    let host = host.trim();
    if host.is_empty() {
        return Err(ValidationError::Empty("хост".into()));
    }
    if host.len() > 253 {
        return Err(ValidationError::TooLong {
            field: "хост".into(),
            max: 253,
        });
    }
    // Разрешаем буквы, цифры, дефис, точку, подчёркивание (для инвенторных имён)
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
    {
        return Err(ValidationError::InvalidHost(host.into()));
    }
    Ok(())
}

/// Валидирует extra_vars как YAML (или пустую строку).
pub fn validate_extra_vars(vars: &str) -> Result<(), ValidationError> {
    let trimmed = vars.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    // Пытаемся распарсить как YAML (один документ)
    serde_yaml::from_str::<serde_yaml::Value>(trimmed)
        .map_err(|e| ValidationError::InvalidYaml(e.to_string()))?;
    Ok(())
}

/// Валидирует список хостов через запятуу.
pub fn validate_hosts(hosts: &str) -> Result<(), ValidationError> {
    if hosts.trim().is_empty() {
        return Err(ValidationError::Empty("хосты".into()));
    }
    for h in hosts.split(',') {
        validate_host(h.trim())?;
    }
    Ok(())
}

/// Нормализует язык интерфейса.
pub fn normalize_lang(input: &str) -> Lang {
    match input.to_lowercase().trim() {
        "ru" | "русский" | "russian" => Lang::Ru,
        _ => Lang::En,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_port_valid() {
        assert_eq!(validate_port("22").unwrap(), 22);
        assert_eq!(validate_port("8080").unwrap(), 8080);
        assert_eq!(validate_port("65535").unwrap(), 65535);
    }

    #[test]
    fn test_validate_port_invalid() {
        assert!(validate_port("0").is_err());
        assert!(validate_port("65536").is_err());
        assert!(validate_port("abc").is_err());
        assert!(validate_port("").is_err());
    }

    #[test]
    fn test_validate_host_valid() {
        validate_host("web1.example.com").unwrap();
        validate_host("192.168.1.1").unwrap();
        validate_host("my-host_01").unwrap();
    }

    #[test]
    fn test_validate_host_invalid() {
        assert!(validate_host("").is_err());
        assert!(validate_host("host with spaces").is_err());
        assert!(validate_host("host;rm -rf /").is_err());
    }

    #[test]
    fn test_validate_extra_vars_valid() {
        validate_extra_vars("").unwrap();
        validate_extra_vars("key: value").unwrap();
        validate_extra_vars("foo: bar\nbaz: 123").unwrap();
    }

    #[test]
    fn test_validate_extra_vars_invalid() {
        assert!(validate_extra_vars(": invalid yaml: [").is_err());
    }

    #[test]
    fn test_validate_hosts() {
        validate_hosts("web1,web2,db1").unwrap();
        validate_hosts("all").unwrap();
        assert!(validate_hosts("").is_err());
        assert!(validate_hosts("bad host;").is_err());
    }
}