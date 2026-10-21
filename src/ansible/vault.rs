pub fn run(action: &str, file: &str, pass_file: Option<&str>) -> Result<String, String> {
    let mut cmd = std::process::Command::new("ansible-vault");
    cmd.arg(action).arg(file);
    if let Some(p) = pass_file {
        cmd.arg("--vault-password-file").arg(p);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}
