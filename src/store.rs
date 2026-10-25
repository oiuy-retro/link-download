//! Profiles live in AppData, not beside the program.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::model::{AudioCodec, AudioQuality, Container, Options, Resolution, VideoCodec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedProfile {
    pub name: String,
    pub options: Options,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    pub last_profile: String,
    pub override_profile: String,
    pub override_location: String,
    pub profiles: Vec<SavedProfile>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            last_profile: "Default".into(),
            override_profile: String::new(),
            override_location: String::new(),
            profiles: vec![SavedProfile { name: "Default".into(), options: Options::default() }],
        }
    }
}

pub fn file_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Link Download").join("profiles.json")
}

pub fn load() -> Memory {
    load_from(&file_path())
}

pub fn save(memory: &Memory) {
    let _ = save_to(&file_path(), memory);
}

pub fn load_from(path: &Path) -> Memory {
    let Ok(text) = fs::read_to_string(path) else {
        return Memory::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Memory::default();
    };
    let mut profiles = Vec::new();
    if let Some(list) = value.get("profiles").and_then(Value::as_array) {
        for entry in list {
            let name = entry.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
            if name.is_empty() {
                continue;
            }
            profiles.push(SavedProfile {
                name,
                options: options_from(entry.get("options").unwrap_or(&Value::Null)),
            });
        }
    }
    if profiles.is_empty() {
        profiles.push(SavedProfile { name: "Default".into(), options: Options::default() });
    }
    let last = value.get("last_profile").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let last_profile = if profiles.iter().any(|profile| profile.name == last) {
        last
    } else {
        profiles[0].name.clone()
    };
    Memory {
        last_profile,
        override_profile: text_at(&value, "override_profile"),
        override_location: text_at(&value, "override_location"),
        profiles,
    }
}

pub fn save_to(path: &Path, memory: &Memory) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let profiles: Vec<Value> = memory
        .profiles
        .iter()
        .map(|profile| json!({ "name": profile.name, "options": options_to_json(&profile.options) }))
        .collect();
    let value = json!({
        "last_profile": memory.last_profile,
        "override_profile": memory.override_profile,
        "override_location": memory.override_location,
        "profiles": profiles,
    });
    let text = serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| err.to_string())
}

fn options_to_json(opts: &Options) -> Value {
    json!({
        "resolution": opts.resolution.key(),
        "video_codec": opts.video_codec.key(),
        "audio_quality": opts.audio_quality.key(),
        "audio_codec": opts.audio_codec.key(),
        "container": opts.container.key(),
        "audio_only": opts.audio_only,
        "reencode": opts.reencode,
        "sponsorblock": opts.sponsorblock,
        "strip_genre": opts.strip_genre,
        "overwrite": opts.overwrite,
        "min_wait": opts.min_wait,
        "max_wait": opts.max_wait,
        "location": opts.location,
    })
}

fn options_from(value: &Value) -> Options {
    Options {
        resolution: Resolution::parse(value.get("resolution").and_then(Value::as_str).unwrap_or("")),
        video_codec: VideoCodec::parse(value.get("video_codec").and_then(Value::as_str).unwrap_or("")),
        audio_quality: AudioQuality::parse(value.get("audio_quality").and_then(Value::as_str).unwrap_or("")),
        audio_codec: AudioCodec::parse(value.get("audio_codec").and_then(Value::as_str).unwrap_or("")),
        container: Container::parse(value.get("container").and_then(Value::as_str).unwrap_or("")),
        audio_only: value.get("audio_only").and_then(Value::as_bool).unwrap_or(false),
        reencode: value.get("reencode").and_then(Value::as_bool).unwrap_or(false),
        sponsorblock: value.get("sponsorblock").and_then(Value::as_bool).unwrap_or(false),
        strip_genre: value.get("strip_genre").and_then(Value::as_bool).unwrap_or(true),
        overwrite: value.get("overwrite").and_then(Value::as_bool).unwrap_or(false),
        min_wait: value.get("min_wait").and_then(Value::as_u64).unwrap_or(4) as u32,
        max_wait: value.get("max_wait").and_then(Value::as_u64).unwrap_or(8) as u32,
        location: text_at(value, "location"),
    }
}

fn text_at(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AudioCodec, Container};
    use serde_json::json;

    #[test]
    fn round_trip_keeps_the_override_with_the_profile() {
        let path = std::env::temp_dir().join(format!("link-download-store-{}.json", std::process::id()));
        let mut memory = Memory::default();
        memory.profiles[0].options.location = "C:\\Music".into();
        memory.profiles[0].options.audio_only = true;
        memory.profiles[0].options.audio_codec = AudioCodec::Opus;
        memory.profiles[0].options.container = Container::M4a;
        memory.profiles[0].options.sponsorblock = true;
        memory.profiles[0].options.strip_genre = false;
        memory.profiles[0].options.overwrite = true;
        memory.last_profile = "Default".into();
        memory.override_profile = "Default".into();
        memory.override_location = "D:\\Incoming".into();
        memory.profiles.push(SavedProfile {
            name: "Quiet".into(),
            options: Options {
                min_wait: 10,
                max_wait: 20,
                ..memory.profiles[0].options.clone()
            },
        });
        save_to(&path, &memory).unwrap();
        let loaded = load_from(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded, memory);
        assert!(!loaded.profiles[0].options.strip_genre);
        assert!(loaded.profiles[0].options.overwrite);
    }

    #[test]
    fn missing_strip_genre_stays_on() {
        let opts = options_from(&json!({ "location": "C:\\Music" }));
        assert!(opts.strip_genre);
        assert!(!opts.sponsorblock);
        assert!(!opts.overwrite, "settings saved before this option existed keep it off");
    }
}
