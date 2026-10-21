//! yt-dlp `--dump-single-json` into one table per pasted link.

use serde_json::Value;

use crate::model::{Block, Group, Item};

pub fn parse_info(json: &str, source_url: &str) -> Result<Group, String> {
    let text = json_object(json).unwrap_or(json.trim());
    let value: Value = serde_json::from_str(text).map_err(|err| format!("The link information was not readable. {err}"))?;
    Ok(group_from_value(&value, source_url))
}

/// yt-dlp can print a warning before the JSON object. Keep the object.
fn json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&text[start..=end])
}

fn group_from_value(value: &Value, source_url: &str) -> Group {
    let entries = value.get("entries").and_then(Value::as_array);
    let is_playlist = entries.is_some() || value.get("_type").and_then(Value::as_str) == Some("playlist");
    if !is_playlist {
        let item = item_from(value, source_url, None, false);
        let album = text_field(value, "album");
        return Group {
            source_url: source_url.to_string(),
            is_playlist: false,
            playlist_title: String::new(),
            album: album.clone(),
            original_album: album,
            playlist_thumbnail: String::new(),
            error: String::new(),
            items: vec![item],
        };
    }
    let mut flat = Vec::new();
    flatten(value, &mut flat);
    let playlist_title = first_text(value, &["title", "playlist_title"]);
    let mut items = Vec::new();
    for (index, entry) in flat.into_iter().enumerate() {
        let number = (index as u32).saturating_add(1);
        match entry {
            None => items.push(blocked_item(number)),
            Some(entry) => {
                let mut item = item_from(entry, source_url, Some(number), true);
                if item.track_number.is_none() {
                    item.track_number = Some(number);
                }
                items.push(item);
            }
        }
    }
    let error = if items.is_empty() { "That link has no videos.".into() } else { String::new() };
    Group {
        source_url: source_url.to_string(),
        is_playlist: true,
        playlist_title: playlist_title.clone(),
        album: playlist_title.clone(),
        original_album: playlist_title,
        playlist_thumbnail: best_thumb(value),
        error,
        items,
    }
}

fn flatten<'a>(value: &'a Value, out: &mut Vec<Option<&'a Value>>) {
    let Some(entries) = value.get("entries").and_then(Value::as_array) else {
        return;
    };
    for entry in entries {
        if entry.is_null() {
            out.push(None);
            continue;
        }
        if entry.get("entries").and_then(Value::as_array).is_some() {
            flatten(entry, out);
        } else {
            out.push(Some(entry));
        }
    }
}

fn blocked_item(number: u32) -> Item {
    Item {
        id: format!("missing-{number}"),
        title: "Unavailable".into(),
        meta_title: "Unavailable".into(),
        original_title: "Unavailable".into(),
        original_artists: Vec::new(),
        artists: Vec::new(),
        download_url: String::new(),
        playlist_item: Some(number),
        track_number: Some(number),
        duration: 0.0,
        thumbnail: String::new(),
        temp_path: None,
        blocked: Some(Block::Missing),
    }
}

fn item_from(value: &Value, source_url: &str, fallback_index: Option<u32>, playlist: bool) -> Item {
    let id = first_text(value, &["id"]);
    let raw_title = first_text(value, &["title", "track"]);
    let title = if raw_title.is_empty() { "Untitled".into() } else { raw_title };
    let artists = artists_from(value);
    let webpage = first_text(value, &["webpage_url", "original_url", "url"]);
    let (download_url, playlist_item) = if webpage.to_ascii_lowercase().starts_with("http://") || webpage.to_ascii_lowercase().starts_with("https://")
    {
        (webpage, None)
    } else if let Some(index) = value.get("playlist_index").and_then(Value::as_u64).map(|n| n as u32).or(fallback_index) {
        (source_url.to_string(), Some(index))
    } else {
        (source_url.to_string(), None)
    };
    let track_number = if playlist {
        value
            .get("playlist_index")
            .and_then(Value::as_u64)
            .map(|n| n as u32)
            .filter(|n| *n > 0)
            .or(fallback_index)
    } else {
        None
    };
    Item {
        id: if id.is_empty() { format!("item-{}", fallback_index.unwrap_or(1)) } else { id },
        title: title.clone(),
        meta_title: title.clone(),
        original_title: title,
        original_artists: artists.clone(),
        artists,
        download_url,
        playlist_item,
        track_number,
        duration: value.get("duration").and_then(Value::as_f64).unwrap_or(0.0),
        thumbnail: best_thumb(value),
        temp_path: None,
        blocked: None,
    }
}

fn artists_from(value: &Value) -> Vec<String> {
    if let Some(list) = value.get("artists").and_then(Value::as_array) {
        let names: Vec<String> = list
            .iter()
            .filter_map(|entry| {
                entry
                    .as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        entry
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                    })
            })
            .collect();
        let names = dedup(names);
        if !names.is_empty() {
            return names;
        }
    }
    let artist = text_field(value, "artist");
    if artist.is_empty() {
        Vec::new()
    } else {
        dedup(split_artist(&artist))
    }
}

fn split_artist(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn dedup(names: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        if out.iter().any(|have: &String| have.eq_ignore_ascii_case(&name)) {
            continue;
        }
        out.push(name);
    }
    out
}

fn best_thumb(value: &Value) -> String {
    let mut best_score = -1i64;
    let mut best = String::new();
    if let Some(list) = value.get("thumbnails").and_then(Value::as_array) {
        for thumb in list {
            let Some(url) = thumb.get("url").and_then(Value::as_str) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }
            let width = thumb.get("width").and_then(Value::as_i64).unwrap_or(0);
            let height = thumb.get("height").and_then(Value::as_i64).unwrap_or(0);
            let score = width.saturating_mul(height);
            if score >= best_score {
                best_score = score;
                best = url.to_string();
            }
        }
    }
    if best.is_empty() {
        text_field(value, "thumbnail")
    } else {
        best
    }
}

fn text_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn first_text(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        let text = text_field(value, key);
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_a_warning_printed_before_the_json() {
        let group = parse_info("WARNING: old\n{\"id\":\"a\",\"title\":\"Song\"}\n", "https://example.com/a").unwrap();
        assert_eq!(group.items[0].title, "Song");
    }

    #[test]
    fn playlist_fills_album_artists_and_track_numbers() {
        let json = r#"{
            "_type": "playlist",
            "title": "News Of The World",
            "thumbnails": [{"url": "https://img.test/pl.jpg", "width": 100, "height": 100}],
            "entries": [
                {
                    "id": "a",
                    "title": "Artist - We Will Rock You (Official)",
                    "webpage_url": "https://www.youtube.com/watch?v=a",
                    "playlist_index": 1,
                    "duration": 120,
                    "artists": ["Queen"],
                    "thumbnails": [{"url": "https://img.test/a.jpg", "width": 10, "height": 10}]
                },
                null,
                {
                    "id": "b",
                    "title": "Under Pressure",
                    "url": "b",
                    "artists": [{"name": "Queen"}, {"name": "David Bowie"}],
                    "artist": "Queen; David Bowie"
                }
            ]
        }"#;
        let group = parse_info(json, "https://www.youtube.com/playlist?list=PL").unwrap();
        assert!(group.is_playlist);
        assert_eq!(group.album, "News Of The World");
        assert_eq!(group.playlist_thumbnail, "https://img.test/pl.jpg");
        assert_eq!(group.items.len(), 3);
        assert_eq!(group.items[0].track_number, Some(1));
        assert_eq!(group.items[0].download_url, "https://www.youtube.com/watch?v=a");
        assert!(group.items[0].playlist_item.is_none());
        assert_eq!(group.items[0].artists, vec!["Queen".to_string()]);
        assert_eq!(group.items[1].blocked, Some(Block::Missing));
        assert_eq!(group.items[1].track_number, Some(2));
        assert_eq!(group.items[2].artists, vec!["Queen".to_string(), "David Bowie".to_string()]);
        assert_eq!(group.items[2].playlist_item, Some(3));
        assert_eq!(group.items[2].track_number, Some(3));
        assert!(group.items[2].download_url.contains("playlist"));
    }

    #[test]
    fn single_video_has_no_track_number_and_ignores_the_channel() {
        let json = r#"{
            "_type": "video",
            "id": "z",
            "title": "Song",
            "webpage_url": "https://www.youtube.com/watch?v=z",
            "uploader": "Some Channel",
            "channel": "Some Channel",
            "album": "Singles",
            "artist": "Queen; Queen"
        }"#;
        let group = parse_info(json, "https://www.youtube.com/watch?v=z").unwrap();
        assert!(!group.is_playlist);
        assert_eq!(group.album, "Singles");
        assert_eq!(group.items[0].track_number, None);
        assert_eq!(group.items[0].artists, vec!["Queen".to_string()]);
        assert!(group.playlist_thumbnail.is_empty());
    }

    #[test]
    fn flat_playlist_entries_keep_watch_urls() {
        let json = r#"{
            "_type": "playlist",
            "title": "OST",
            "entries": [
                {"_type": "url", "id": "a", "title": "Track 1", "url": "https://www.youtube.com/watch?v=a", "duration": 12},
                {"_type": "url", "id": "b", "title": "Track 2", "url": "https://www.youtube.com/watch?v=b"}
            ]
        }"#;
        let group = parse_info(json, "https://www.youtube.com/playlist?list=PL").unwrap();
        assert!(group.is_playlist);
        assert_eq!(group.album, "OST");
        assert_eq!(group.items.len(), 2);
        assert_eq!(group.items[0].title, "Track 1");
        assert_eq!(group.items[0].download_url, "https://www.youtube.com/watch?v=a");
        assert!(group.items[0].playlist_item.is_none());
        assert_eq!(group.items[1].track_number, Some(2));
    }
}
