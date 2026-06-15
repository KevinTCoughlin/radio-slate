use std::collections::HashMap;
use std::path::Path;

use crate::domain::Station;

// ── Public API ────────────────────────────────────────────────────────────────

/// Import stations from `path`, auto-detecting the format by file extension.
///
/// Supported extensions: `.pls`, `.m3u`, `.m3u8`, `.json`.
pub fn import_from_path(path: impl AsRef<Path>) -> anyhow::Result<Vec<Station>> {
    let path = path.as_ref();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let content = std::fs::read_to_string(path)?;
    match ext.as_str() {
        "pls" => parse_pls(&content),
        "m3u" | "m3u8" => parse_m3u(&content),
        "json" => parse_json(&content),
        other => anyhow::bail!("unsupported import format '.{other}'; expected pls, m3u, or json"),
    }
}

/// Parse a PLS playlist and return the contained stations.
///
/// The genre of every imported station is set to `"imported"`.
/// Entries where the URL fails validation are silently skipped.
pub fn parse_pls(content: &str) -> anyhow::Result<Vec<Station>> {
    let mut files: HashMap<u32, String> = HashMap::new();
    let mut titles: HashMap<u32, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("File")
            && let Some(eq) = rest.find('=')
            && let Ok(idx) = rest[..eq].parse::<u32>()
        {
            let url = rest[eq + 1..].trim().to_string();
            if !url.is_empty() {
                files.insert(idx, url);
            }
        } else if let Some(rest) = line.strip_prefix("Title")
            && let Some(eq) = rest.find('=')
            && let Ok(idx) = rest[..eq].parse::<u32>()
        {
            let title = rest[eq + 1..].trim().to_string();
            if !title.is_empty() {
                titles.insert(idx, title);
            }
        }
    }

    let mut indices: Vec<u32> = files.keys().copied().collect();
    indices.sort_unstable();

    let mut stations = Vec::new();
    for idx in indices {
        let url = &files[&idx];
        let name = titles
            .get(&idx)
            .cloned()
            .unwrap_or_else(|| url_to_name(url));
        if let Ok(station) = Station::new(&name, url, "imported") {
            stations.push(station);
        }
    }
    Ok(stations)
}

/// Parse an M3U / M3U8 playlist and return the contained stations.
///
/// Extended M3U (`#EXTINF`) metadata is used for the station name when
/// present.  Plain M3U files (no `#EXTINF` lines) are also supported; the
/// station name is derived from the stream URL in that case.
/// Entries where the URL fails validation are silently skipped.
pub fn parse_m3u(content: &str) -> anyhow::Result<Vec<Station>> {
    let mut stations = Vec::new();
    let mut pending_name: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line == "#EXTM3U" {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            // Format: #EXTINF:<duration>,<title>
            if let Some(comma) = info.find(',') {
                let title = info[comma + 1..].trim().to_string();
                if !title.is_empty() {
                    pending_name = Some(title);
                }
            }
        } else if !line.starts_with('#') {
            let url = line.to_string();
            let name = pending_name.take().unwrap_or_else(|| url_to_name(&url));
            if let Ok(station) = Station::new(&name, &url, "imported") {
                stations.push(station);
            }
        }
    }
    Ok(stations)
}

/// Export `stations` as a pretty-printed JSON array.
pub fn export_to_json(stations: &[Station]) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(stations)?)
}

/// Export `stations` as an Extended M3U playlist string.
pub fn export_to_m3u(stations: &[Station]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for s in stations {
        out.push_str(&format!("#EXTINF:-1,{}\n{}\n", s.name, s.url));
    }
    out
}

/// Write the station list to `path`, choosing the format by file extension.
///
/// Supported extensions: `.json`, `.m3u`, `.m3u8`.
pub fn export_to_path(stations: &[Station], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let content = match ext.as_str() {
        "json" => export_to_json(stations)?,
        "m3u" | "m3u8" => export_to_m3u(stations),
        other => anyhow::bail!("unsupported export format '.{other}'; expected json or m3u"),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Derive a human-readable name from a stream URL.
fn url_to_name(url: &str) -> String {
    url.split('/')
        .next_back()
        .unwrap_or(url)
        .split('?')
        .next()
        .unwrap_or(url)
        .to_string()
}

/// Parse a JSON array of stations (used for JSON import).
fn parse_json(content: &str) -> anyhow::Result<Vec<Station>> {
    let stations: Vec<Station> = serde_json::from_str(content)?;
    Ok(stations)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── PLS ──────────────────────────────────────────────────────────────────

    #[test]
    fn parse_pls_extracts_stations() {
        let pls = "\
[playlist]
NumberOfEntries=2

File1=http://stream1.example.test/radio
Title1=Test Radio One

File2=https://stream2.example.test/live
Title2=Test Radio Two

Version=2
";
        let stations = parse_pls(pls).unwrap();
        assert_eq!(stations.len(), 2);
        assert_eq!(stations[0].name, "Test Radio One");
        assert_eq!(stations[0].url, "http://stream1.example.test/radio");
        assert_eq!(stations[1].name, "Test Radio Two");
    }

    #[test]
    fn parse_pls_falls_back_to_url_derived_name() {
        let pls = "File1=http://stream.example.test/audio.mp3\n";
        let stations = parse_pls(pls).unwrap();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, "audio.mp3");
    }

    #[test]
    fn parse_pls_skips_entries_with_invalid_url() {
        let pls = "File1=not-a-url\nTitle1=Bad\n";
        let stations = parse_pls(pls).unwrap();
        assert!(stations.is_empty());
    }

    // ── M3U ──────────────────────────────────────────────────────────────────

    #[test]
    fn parse_m3u_extended_extracts_stations() {
        let m3u = "\
#EXTM3U
#EXTINF:-1,Echo Jazz
http://jazz.example.test/stream
#EXTINF:-1,World Beat FM
https://world.example.test/live
";
        let stations = parse_m3u(m3u).unwrap();
        assert_eq!(stations.len(), 2);
        assert_eq!(stations[0].name, "Echo Jazz");
        assert_eq!(stations[1].url, "https://world.example.test/live");
    }

    #[test]
    fn parse_m3u_plain_derives_name_from_url() {
        let m3u = "#EXTM3U\nhttp://stream.example.test/plain.mp3\n";
        let stations = parse_m3u(m3u).unwrap();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, "plain.mp3");
    }

    #[test]
    fn parse_m3u_skips_entries_with_invalid_url() {
        let m3u = "#EXTM3U\n#EXTINF:-1,Bad\nnot-a-url\n";
        let stations = parse_m3u(m3u).unwrap();
        assert!(stations.is_empty());
    }

    // ── JSON ─────────────────────────────────────────────────────────────────

    #[test]
    fn parse_json_round_trips_stations() {
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        let json = serde_json::to_string(&vec![station.clone()]).unwrap();
        let stations = parse_json(&json).unwrap();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, "Echo");
    }

    // ── Export ───────────────────────────────────────────────────────────────

    #[test]
    fn export_to_json_produces_valid_json() {
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        let json = export_to_json(&[station]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn export_to_m3u_includes_extinf_and_url() {
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        let m3u = export_to_m3u(&[station]);
        assert!(m3u.contains("#EXTM3U"));
        assert!(m3u.contains("#EXTINF:-1,Echo"));
        assert!(m3u.contains("https://example.test/stream"));
    }

    #[test]
    fn export_to_path_writes_json_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("export.json");
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        export_to_path(&[station], &path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Echo"));
    }

    #[test]
    fn export_to_path_writes_m3u_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("export.m3u");
        let station = Station::new("Echo", "https://example.test/stream", "jazz").unwrap();
        export_to_path(&[station], &path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("#EXTM3U"));
    }

    #[test]
    fn export_to_path_rejects_unsupported_extension() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("export.csv");
        let result = export_to_path(&[], &path);
        assert!(result.is_err());
    }

    // ── Import from path ─────────────────────────────────────────────────────

    #[test]
    fn import_from_path_dispatches_by_extension() {
        let dir = TempDir::new().unwrap();
        let pls_path = dir.path().join("list.pls");
        fs::write(
            &pls_path,
            "File1=http://example.test/stream\nTitle1=Path Test\n",
        )
        .unwrap();
        let stations = import_from_path(&pls_path).unwrap();
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, "Path Test");
    }

    #[test]
    fn import_from_path_rejects_unknown_extension() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("list.txt");
        fs::write(&path, "some content").unwrap();
        let result = import_from_path(&path);
        assert!(result.is_err());
    }
}
