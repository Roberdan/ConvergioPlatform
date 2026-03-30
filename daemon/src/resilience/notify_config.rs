use crate::resilience::notify::ChannelConfig;

#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub macos_enabled: bool,
    pub ntfy_enabled: bool,
    pub ntfy_server: String,
    pub ntfy_topic: String,
    pub dashboard_enabled: bool,
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            macos_enabled: false,
            ntfy_enabled: false,
            ntfy_server: "https://ntfy.sh".to_string(),
            ntfy_topic: "convergio-mesh".to_string(),
            dashboard_enabled: true,
            telegram_enabled: false,
            telegram_bot_token: None,
            telegram_chat_id: None,
        }
    }
}

impl NotificationSettings {
    pub fn load() -> Self {
        let Some(path) = candidate_paths()
            .into_iter()
            .find(|path| path.exists()) else {
            return Self::default();
        };

        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        parse_notifications_conf(&content)
    }

    pub fn enabled_channel_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.macos_enabled {
            names.push("macos");
        }
        if self.ntfy_enabled {
            names.push("ntfy");
        }
        if self.telegram_enabled {
            names.push("telegram");
        }
        if self.dashboard_enabled {
            names.push("dashboard");
        }
        names
    }

    pub fn channel_config(&self, name: &str) -> Result<Option<ChannelConfig>, String> {
        match name {
            "macos" => Ok(self.macos_enabled.then_some(ChannelConfig::MacOS)),
            "ntfy" => Ok(self.ntfy_enabled.then_some(ChannelConfig::Ntfy {
                topic: self.ntfy_topic.clone(),
                base_url: self.ntfy_server.clone(),
            })),
            "telegram" => {
                if !self.telegram_enabled {
                    return Ok(None);
                }
                let token = self
                    .telegram_bot_token
                    .clone()
                    .or_else(crate::telegram_config::telegram_token)
                    .ok_or_else(|| {
                        "telegram enabled but no bot token configured".to_string()
                    })?;
                let chat_id = self
                    .telegram_chat_id
                    .clone()
                    .or_else(crate::telegram_config::telegram_chat_id_raw)
                    .ok_or_else(|| "telegram enabled but no chat_id configured".to_string())?;
                Ok(Some(ChannelConfig::Telegram { bot_token: token, chat_id }))
            }
            _ => Ok(None),
        }
    }
}

fn candidate_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("claude-config/config/notifications.conf"));
        paths.push(cwd.join("config/notifications.conf"));
        if let Some(parent) = cwd.parent() {
            paths.push(parent.join("claude-config/config/notifications.conf"));
            paths.push(parent.join("config/notifications.conf"));
        }
    }
    paths
}

fn parse_bool(value: &str) -> bool {
    matches!(value.trim(), "true" | "1" | "yes" | "on")
}

fn parse_notifications_conf(content: &str) -> NotificationSettings {
    let mut settings = NotificationSettings::default();
    let mut section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match (section.as_str(), key) {
            ("macos", "enabled") => settings.macos_enabled = parse_bool(value),
            ("ntfy", "enabled") => settings.ntfy_enabled = parse_bool(value),
            ("ntfy", "server") => settings.ntfy_server = value.to_string(),
            ("ntfy", "topic") => settings.ntfy_topic = value.to_string(),
            ("dashboard", "enabled") => settings.dashboard_enabled = parse_bool(value),
            ("telegram", "enabled") => settings.telegram_enabled = parse_bool(value),
            ("telegram", "bot_token") if !value.is_empty() => {
                settings.telegram_bot_token = Some(value.to_string());
            }
            ("telegram", "chat_id") if !value.is_empty() => {
                settings.telegram_chat_id = Some(value.to_string());
            }
            _ => {}
        }
    }

    settings
}

#[cfg(test)]
mod tests {
    use super::parse_notifications_conf;

    #[test]
    fn parse_notifications_conf_reads_enabled_channels() {
        let settings = parse_notifications_conf(
            "[macos]\nenabled=true\n[ntfy]\nenabled=true\nserver=https://ntfy.sh\ntopic=test\n\
             [dashboard]\nenabled=true\n[telegram]\nenabled=true\nbot_token=abc\nchat_id=42\n",
        );
        assert!(settings.macos_enabled);
        assert!(settings.ntfy_enabled);
        assert!(settings.dashboard_enabled);
        assert!(settings.telegram_enabled);
        assert_eq!(settings.ntfy_topic, "test");
        assert_eq!(settings.telegram_chat_id.as_deref(), Some("42"));
    }
}
