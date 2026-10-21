use crate::models::Host;
use anyhow::Result;
use std::path::Path;

pub fn parse_inventory(path: &Path) -> Result<Vec<Host>> {
    let content = std::fs::read_to_string(path)?;
    let mut hosts = Vec::new();
    let mut group = "ungrouped".to_string();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            group = line[1..line.len() - 1].to_string();
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(name) = parts.first() {
            let mut h = Host {
                name: name.to_string(),
                group: group.clone(),
                ..Default::default()
            };
            for kv in parts.iter().skip(1) {
                if let Some((k, v)) = kv.split_once('=') {
                    match k {
                        "ansible_host" => h.ansible_host = Some(v.into()),
                        "ansible_user" => h.ansible_user = Some(v.into()),
                        _ => {
                            h.vars.insert(k.into(), v.into());
                        }
                    }
                }
            }
            hosts.push(h);
        }
    }
    Ok(hosts)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inventory() {
        let f = tempfile_named("inv.ini");
        std::fs::write(
            &f,
            "[web]\nweb1 ansible_host=1.2.3.4 ansible_user=admin\n\ndb1\n",
        )
        .unwrap();
        let hosts = parse_inventory(&f).unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].group, "web");
        assert_eq!(hosts[0].ansible_host.as_deref(), Some("1.2.3.4"));
        assert_eq!(hosts[1].group, "web");
    }

    #[test]
    fn test_empty_inventory() {
        let f = tempfile_named("empty.ini");
        let hosts = parse_inventory(&f).unwrap();
        assert!(hosts.is_empty());
    }

    fn tempfile_named(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(name);
        std::fs::File::create(&p).unwrap();
        p
    }
}
