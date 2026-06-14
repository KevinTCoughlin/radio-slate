#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StationMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub bitrate_kbps: Option<u32>,
}

pub fn parse_stream_title(value: &str) -> Option<StationMetadata> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((artist, title)) = trimmed.split_once(" - ") {
        let artist = artist.trim();
        let title = title.trim();
        return Some(StationMetadata {
            title: (!title.is_empty()).then(|| title.to_string()),
            artist: (!artist.is_empty()).then(|| artist.to_string()),
            bitrate_kbps: None,
        });
    }

    Some(StationMetadata {
        title: Some(trimmed.to_string()),
        artist: None,
        bitrate_kbps: None,
    })
}

pub fn parse_bitrate_from_url(url: &str) -> Option<u32> {
    let mut best: Option<u32> = None;
    let mut digits = String::new();

    for ch in url.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }

        if !digits.is_empty() {
            if let Ok(value) = digits.parse::<u32>() {
                if (8..=1024).contains(&value) {
                    best = Some(value);
                }
            }
            digits.clear();
        }
    }

    if !digits.is_empty() {
        if let Ok(value) = digits.parse::<u32>() {
            if (8..=1024).contains(&value) {
                best = Some(value);
            }
        }
    }

    best
}

pub fn format_metadata(metadata: &StationMetadata) -> String {
    let mut parts = Vec::new();

    if let Some(title) = &metadata.title {
        parts.push(format!("title: {title}"));
    }
    if let Some(artist) = &metadata.artist {
        parts.push(format!("artist: {artist}"));
    }
    if let Some(bitrate) = metadata.bitrate_kbps {
        parts.push(format!("bitrate: {bitrate} kbps"));
    }

    if parts.is_empty() {
        "unavailable".to_string()
    } else {
        parts.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::{format_metadata, parse_bitrate_from_url, parse_stream_title};

    #[test]
    fn parses_artist_and_title_metadata() {
        let metadata = parse_stream_title("Boards of Canada - Dayvan Cowboy").unwrap();
        assert_eq!(metadata.artist.as_deref(), Some("Boards of Canada"));
        assert_eq!(metadata.title.as_deref(), Some("Dayvan Cowboy"));
    }

    #[test]
    fn parses_bitrate_from_station_url() {
        assert_eq!(
            parse_bitrate_from_url("http://live-mp3-128.kexp.org/kexp128.mp3"),
            Some(128)
        );
    }

    #[test]
    fn formats_metadata_with_fallback() {
        let unavailable = format_metadata(&Default::default());
        assert_eq!(unavailable, "unavailable");
    }
}
