pub fn list_rpms(dir: &str) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.path().extension().and_then(|s| s.to_str()) == Some("rpm") {
                v.push(e.file_name().to_string_lossy().into_owned());
            }
        }
    }
    v.sort();
    v
}

pub fn createrepo(dir: &str) -> Result<String, String> {
    let out = std::process::Command::new("createrepo")
        .arg(dir)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}
