//! The control channel's key file, one rule for the app (which writes it) and the MCP bridge
//! (which reads it): `<settings>/control-key.<instance>`. See `docs/control-protocol.md`, Keys.

use std::path::{Path, PathBuf};

/// Name of the folder that makes the app portable: when it sits next to the program, everything
/// the app keeps (preferences, personal dictionary, fonts, recovery copies, logs, key files and
/// the `dictionaries` folder) lives in it instead of the user's profile. The Hunspell search in
/// `wordcraft-proof` looks for the same name.
pub const PORTABLE_DIR: &str = "wordbook-data";

/// The portable data folder for a program in `exe_dir`: `exe_dir/wordbook-data`, only when it
/// exists (creating the folder is how portable mode is switched on).
pub fn portable_dir_in(exe_dir: &Path) -> Option<PathBuf> {
    let dir = exe_dir.join(PORTABLE_DIR);
    dir.is_dir().then_some(dir)
}

/// [`portable_dir_in`] for the running program's own folder.
pub fn portable_dir() -> Option<PathBuf> {
    portable_dir_in(std::env::current_exe().ok()?.parent()?)
}

/// The app's settings folder (preferences, logs, key files): the [`PORTABLE_DIR`] folder next to
/// the program when there is one, else macOS `~/Library/Application Support/WordCraft`, Windows
/// `%APPDATA%\WordCraft`, otherwise `$XDG_CONFIG_HOME/wordcraft` or `~/.config/wordcraft`.
/// `WORDCRAFT_PROFILE_DIR` overrides the entire folder for isolated runs.
pub fn settings_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("WORDCRAFT_PROFILE_DIR").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    if let Some(dir) = portable_dir() {
        return Some(dir);
    }
    if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/WordCraft"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("WordCraft"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|c| c.join("wordcraft"))
    }
}

/// The `instance-id=` value of a `/.flatpak-info` file, only when it is `[A-Za-z0-9_-]+` (it
/// becomes part of a file name).
pub fn parse_instance_id(info: &str) -> Option<String> {
    let v = info.lines().find_map(|l| l.strip_prefix("instance-id="))?.trim();
    (!v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')).then(|| v.to_string())
}

/// The Flatpak instance id, or `None` outside Flatpak.
pub fn instance_id() -> Option<String> {
    parse_instance_id(&std::fs::read_to_string("/.flatpak-info").ok()?)
}

/// `<settings>/control-key.<instance>`, with `<instance>` the Flatpak instance id or
/// `local-<port>`.
pub fn key_path(settings: &Path, port: u16) -> PathBuf {
    key_path_for(settings, instance_id().as_deref(), port)
}

fn key_path_for(settings: &Path, instance: Option<&str>, port: u16) -> PathBuf {
    match instance {
        Some(id) => settings.join(format!("control-key.{id}")),
        None => settings.join(format!("control-key.local-{port}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn instance_id_only_safe_values() {
        assert_eq!(parse_instance_id("[Instance]\ninstance-id=abc_123-X\n").as_deref(), Some("abc_123-X"));
        assert_eq!(parse_instance_id("instance-id= 42 \n").as_deref(), Some("42"));
        // It ends up in a file name.
        assert_eq!(parse_instance_id("instance-id=../etc\n"), None);
        assert_eq!(parse_instance_id("instance-id=a/b\n"), None);
        assert_eq!(parse_instance_id("instance-id=a b\n"), None);
        assert_eq!(parse_instance_id("instance-id=..\n"), None);
        assert_eq!(parse_instance_id("instance-id=\n"), None);
        assert_eq!(parse_instance_id("app=ai.storyteller.wordcraft\n"), None);
    }

    #[test]
    fn key_file_per_instance() {
        let dir = Path::new("/settings/wordcraft");
        assert_eq!(key_path_for(dir, Some("1234"), 7981), dir.join("control-key.1234"));
        assert_eq!(key_path_for(dir, None, 7981), dir.join("control-key.local-7981"));
        assert_eq!(key_path_for(dir, None, 7982), dir.join("control-key.local-7982"));
        if instance_id().is_none() {
            assert_eq!(key_path(dir, 7981), dir.join("control-key.local-7981"));
        }
    }

    #[test]
    fn a_data_folder_next_to_the_program_makes_it_portable() {
        let exe_dir = std::env::temp_dir().join(format!("wordbook-portable-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&exe_dir);
        std::fs::create_dir_all(&exe_dir).unwrap();
        assert_eq!(portable_dir_in(&exe_dir), None, "no folder, not portable");
        std::fs::write(exe_dir.join(PORTABLE_DIR), b"a file is not a folder").unwrap();
        assert_eq!(portable_dir_in(&exe_dir), None, "a file of that name doesn't count");
        std::fs::remove_file(exe_dir.join(PORTABLE_DIR)).unwrap();
        std::fs::create_dir(exe_dir.join(PORTABLE_DIR)).unwrap();
        assert_eq!(portable_dir_in(&exe_dir), Some(exe_dir.join(PORTABLE_DIR)));
        let _ = std::fs::remove_dir_all(&exe_dir);
    }

    #[test]
    fn settings_dir_is_the_apps_folder() {
        if let Some(d) = settings_dir() {
            let name = d.file_name().and_then(|n| n.to_str()).unwrap_or("");
            assert!(name == "wordcraft" || name == "WordCraft", "{}", d.display());
        }
    }
}
