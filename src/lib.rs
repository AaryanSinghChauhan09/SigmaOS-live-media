//! SigmaOS live-media — live wallpaper / animated background manager.
//!
//! Manages animated wallpapers (video, shader, or image-sequence) for the
//! SigmaOS desktop. Declarative TOML configuration, atomic apply, pause on
//! battery saver.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Errors produced by this component.
#[derive(Debug, thiserror::Error)]
pub enum LiveMediaError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(PathBuf),
}

/// Type of live media backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    /// Video file (mp4/webm) played via mpvpaper or similar.
    Video,
    /// GLSL shader wallpaper.
    Shader,
    /// Image sequence (directory of frames).
    Sequence,
    /// Static image fallback.
    Image,
}

/// Playback configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playback {
    /// Loop the media.
    #[serde(default = "default_true")]
    pub loop_media: bool,
    /// Volume 0-100 (video only).
    #[serde(default)]
    pub volume: u8,
    /// Pause when on battery.
    #[serde(default)]
    pub pause_on_battery: bool,
}

fn default_true() -> bool {
    true
}

/// A single wallpaper entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallpaper {
    pub name: String,
    pub kind: MediaKind,
    /// Path to media file or directory (relative to wallpapers dir or absolute).
    pub path: PathBuf,
    #[serde(default)]
    pub playback: Option<Playback>,
}

/// Root configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMediaConfig {
    /// Directory containing wallpaper assets.
    pub wallpapers_dir: PathBuf,
    /// Currently active wallpaper name.
    pub active: String,
    /// All known wallpapers.
    pub wallpapers: Vec<Wallpaper>,
}

impl LiveMediaConfig {
    /// Load config from TOML.
    pub fn load(path: &Path) -> Result<Self, LiveMediaError> {
        let text = std::fs::read_to_string(path)
            .map_err(|_| LiveMediaError::NotFound(path.to_path_buf()))?;
        let cfg: LiveMediaConfig = toml::from_str(&text)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), LiveMediaError> {
        if self.wallpapers.is_empty() {
            return Err(LiveMediaError::Validation("no wallpapers defined".into()));
        }
        if !self.wallpapers.iter().any(|w| w.name == self.active) {
            return Err(LiveMediaError::Validation(format!(
                "active wallpaper '{}' not found",
                self.active
            )));
        }
        for w in &self.wallpapers {
            if w.name.is_empty() {
                return Err(LiveMediaError::Validation(
                    "wallpaper name cannot be empty".into(),
                ));
            }
        }
        Ok(())
    }

    /// Get the active wallpaper.
    pub fn active_wallpaper(&self) -> Option<&Wallpaper> {
        self.wallpapers.iter().find(|w| w.name == self.active)
    }

    /// Resolve a wallpaper's media path (expands ~ and joins wallpapers_dir).
    pub fn resolve_path(&self, w: &Wallpaper) -> PathBuf {
        if w.path.is_absolute() {
            w.path.clone()
        } else {
            self.wallpapers_dir.join(&w.path)
        }
    }

    /// Serialize to TOML.
    pub fn to_toml(&self) -> Result<String, LiveMediaError> {
        Ok(toml::to_string(self)?)
    }

    /// Apply: write config atomically to config dir.
    pub fn apply(&self, config_dir: &Path) -> Result<PathBuf, LiveMediaError> {
        self.validate()?;
        std::fs::create_dir_all(config_dir)?;
        let path = config_dir.join("live-media.toml");
        let tmp = config_dir.join("live-media.toml.tmp");
        std::fs::write(&tmp, self.to_toml()?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }
}

impl Default for LiveMediaConfig {
    fn default() -> Self {
        Self {
            wallpapers_dir: PathBuf::from("~/.local/share/sigmaos/wallpapers"),
            active: "default".into(),
            wallpapers: vec![Wallpaper {
                name: "default".into(),
                kind: MediaKind::Image,
                path: PathBuf::from("default.png"),
                playback: None,
            }],
        }
    }
}

/// Standard config path.
pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.config_dir().join("live-media.toml"))
}

/// Standard config directory.
pub fn default_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> LiveMediaConfig {
        LiveMediaConfig {
            wallpapers_dir: PathBuf::from("/wp"),
            active: "neon".into(),
            wallpapers: vec![
                Wallpaper {
                    name: "neon".into(),
                    kind: MediaKind::Shader,
                    path: PathBuf::from("neon.glsl"),
                    playback: Some(Playback {
                        loop_media: true,
                        volume: 50,
                        pause_on_battery: true,
                    }),
                },
                Wallpaper {
                    name: "abs".into(),
                    kind: MediaKind::Video,
                    path: PathBuf::from("/media/clip.mp4"),
                    playback: None,
                },
            ],
        }
    }

    #[test]
    fn validate_accepts_good_config() {
        assert!(cfg().validate().is_ok());
        assert!(LiveMediaConfig::default().validate().is_ok());
    }

    #[test]
    fn validate_rejects_bad_configs() {
        let mut c = cfg();
        c.wallpapers.clear();
        assert!(matches!(c.validate(), Err(LiveMediaError::Validation(_))));
        let mut c = cfg();
        c.active = "ghost".into();
        assert!(c.validate().unwrap_err().to_string().contains("ghost"));
        let mut c = cfg();
        c.wallpapers[0].name.clear();
        assert!(matches!(c.validate(), Err(LiveMediaError::Validation(_))));
    }

    #[test]
    fn active_and_resolve_path() {
        let c = cfg();
        let w = c.active_wallpaper().unwrap();
        assert_eq!(w.name, "neon");
        // Relative path joins wallpapers_dir; absolute stays absolute.
        assert_eq!(c.resolve_path(w), PathBuf::from("/wp/neon.glsl"));
        let abs = c.wallpapers.iter().find(|w| w.name == "abs").unwrap();
        assert_eq!(c.resolve_path(abs), PathBuf::from("/media/clip.mp4"));
    }

    #[test]
    fn toml_roundtrip_and_apply() {
        let dir = tempfile::tempdir().unwrap();
        let c = cfg();
        let toml = c.to_toml().unwrap();
        let back: LiveMediaConfig = toml::from_str(&toml).unwrap();
        assert_eq!(back.active, "neon");
        assert_eq!(back.wallpapers.len(), 2);
        assert_eq!(back.wallpapers[0].playback.as_ref().unwrap().volume, 50);
        // apply writes atomically and load reads it back.
        let cfg_dir = dir.path().join("cfg");
        let written = c.apply(&cfg_dir).unwrap();
        assert!(written.ends_with("live-media.toml"));
        let loaded = LiveMediaConfig::load(&written).unwrap();
        assert_eq!(loaded.active, "neon");
        assert!(LiveMediaConfig::load(&dir.path().join("missing.toml")).is_err());
    }
}
