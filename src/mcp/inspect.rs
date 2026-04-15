use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

pub fn read_save_metadata(
    home_path: &Path,
    max_entries: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let saves_root = home_path.join(".polit").join("saves");
    let mut entries = Vec::new();
    collect_files(&saves_root, &saves_root, &mut entries)?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries.truncate(max_entries.max(1));

    Ok(json!({
        "saveEntries": entries
            .into_iter()
            .map(|(path, bytes)| json!({ "path": path, "bytes": bytes }))
            .collect::<Vec<_>>(),
    }))
}

pub fn read_recent_logs(
    home_path: &Path,
    log_kind: &str,
    max_lines: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let sanitized = sanitize_component(log_kind)?;
    let log_path = home_path
        .join(".polit")
        .join("logs")
        .join(format!("{sanitized}.log"));
    let content = fs::read_to_string(&log_path)?;
    let lines = tail_lines(&content, max_lines);

    Ok(json!({
        "logKind": sanitized,
        "lines": lines,
    }))
}

pub fn read_file_excerpt(
    home_path: &Path,
    relative_path: &str,
    max_lines: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let full_path = resolve_whitelisted_path(home_path, relative_path)?;
    let content = fs::read_to_string(&full_path)?;
    let lines = content
        .lines()
        .take(max_lines.max(1))
        .map(str::to_string)
        .collect::<Vec<_>>();

    Ok(json!({
        "path": full_path,
        "lines": lines,
    }))
}

fn resolve_whitelisted_path(
    home_path: &Path,
    relative_path: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("absolute paths are not allowed".into());
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("path escapes the allowed POLIT directories".into());
    }

    let full_path = home_path.join(path);
    let allowed_roots = [
        home_path.join(".polit").join("config"),
        home_path.join(".polit").join("logs"),
        home_path.join(".polit").join("saves"),
    ];

    if allowed_roots
        .iter()
        .any(|allowed| full_path.starts_with(allowed))
    {
        Ok(full_path)
    } else {
        Err("path is outside the allowed POLIT config/log/save roots".into())
    }
}

fn sanitize_component(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    if value.is_empty() {
        return Err("value cannot be empty".into());
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(value.to_string())
    } else {
        Err("value contains unsupported characters".into())
    }
}

fn tail_lines(content: &str, max_lines: usize) -> Vec<String> {
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let keep = max_lines.max(1);
    if lines.len() > keep {
        lines.drain(0..lines.len() - keep);
    }
    lines
}

fn collect_files(
    root: &Path,
    current: &Path,
    entries: &mut Vec<(String, u64)>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !current.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_files(root, &path, entries)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)?
                .display()
                .to_string();
            entries.push((relative, metadata.len()));
        }
    }

    Ok(())
}
