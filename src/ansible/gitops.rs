pub fn run(dir: &str, args: &[&str]) -> String {
    match std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
    {
        Ok(o) => format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => format!("git error: {}", e),
    }
}
