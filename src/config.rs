//! 設定ファイルとスタートアップ登録。C ドライブを汚さないよう exe の隣の data フォルダに保存する。

use std::{
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "MicSwitch";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 自動切り替えの対象 (★) にしたデバイス名。USB ポートを変えても一致するよう ID ではなく名前で持つ。
    pub starred: Vec<String>,
    pub auto_switch: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { starred: Vec::new(), auto_switch: true }
    }
}

/// exe の隣に書き込めるならそこ (ポータブル運用)、書き込めなければ %LOCALAPPDATA%\MicSwitch。
pub fn data_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let portable = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("data")));
        if let Some(dir) = portable.filter(|dir| is_writable(dir)) {
            return dir;
        }
        std::env::var_os("LOCALAPPDATA")
            .map(|base| PathBuf::from(base).join("MicSwitch"))
            .unwrap_or_else(|| PathBuf::from("data"))
    })
    .clone()
}

fn is_writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    let ok = fs::write(&probe, b"").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

impl Config {
    pub fn load() -> Self {
        fs::read_to_string(config_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = fs::create_dir_all(data_dir());
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(config_path(), text);
        }
    }
}

fn reg(args: &[&str]) -> bool {
    Command::new("reg")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn autostart_enabled() -> bool {
    reg(&["query", RUN_KEY, "/v", RUN_VALUE])
}

pub fn set_autostart(enabled: bool) -> bool {
    if enabled {
        let Ok(exe) = std::env::current_exe() else { return false };
        let command = format!("\"{}\" --hidden", exe.display());
        reg(&["add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &command, "/f"])
    } else {
        reg(&["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
    }
}
