fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty()) // intentional: missing env means Telegram is not configured
}

pub fn telegram_token() -> Option<String> {
    env_non_empty("CONVERGIO_TELEGRAM_TOKEN")
        .or_else(|| env_non_empty("TELEGRAM_BOT_TOKEN"))
}

pub fn telegram_chat_id_raw() -> Option<String> {
    env_non_empty("CONVERGIO_TELEGRAM_CHAT_ID")
        .or_else(|| env_non_empty("TELEGRAM_CHAT_ID"))
}

pub fn telegram_chat_id() -> Result<Option<i64>, String> {
    match telegram_chat_id_raw() {
        Some(value) => value
            .parse::<i64>()
            .map(Some)
            .map_err(|err| err.to_string()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::{telegram_chat_id, telegram_token};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn snapshot(name: &str) -> Option<String> {
        std::env::var(name).ok() // intentional: test helper snapshots absent vars as None
    }

    fn restore(name: &str, value: Option<String>) {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }

    #[test]
    fn canonical_env_wins_over_legacy_alias() {
        let _guard = env_lock().lock().unwrap();
        let canonical_before = snapshot("CONVERGIO_TELEGRAM_TOKEN");
        let legacy_before = snapshot("TELEGRAM_BOT_TOKEN");
        std::env::set_var("CONVERGIO_TELEGRAM_TOKEN", "canonical");
        std::env::set_var("TELEGRAM_BOT_TOKEN", "legacy");
        assert_eq!(telegram_token().as_deref(), Some("canonical"));
        restore("CONVERGIO_TELEGRAM_TOKEN", canonical_before);
        restore("TELEGRAM_BOT_TOKEN", legacy_before);
    }

    #[test]
    fn legacy_aliases_are_supported() {
        let _guard = env_lock().lock().unwrap();
        let canonical_before = snapshot("CONVERGIO_TELEGRAM_TOKEN");
        let legacy_before = snapshot("TELEGRAM_BOT_TOKEN");
        let canonical_chat_before = snapshot("CONVERGIO_TELEGRAM_CHAT_ID");
        let legacy_chat_before = snapshot("TELEGRAM_CHAT_ID");
        std::env::remove_var("CONVERGIO_TELEGRAM_TOKEN");
        std::env::remove_var("CONVERGIO_TELEGRAM_CHAT_ID");
        std::env::set_var("TELEGRAM_BOT_TOKEN", "legacy");
        std::env::set_var("TELEGRAM_CHAT_ID", "42");
        assert_eq!(telegram_token().as_deref(), Some("legacy"));
        assert_eq!(telegram_chat_id().unwrap(), Some(42));
        restore("CONVERGIO_TELEGRAM_TOKEN", canonical_before);
        restore("TELEGRAM_BOT_TOKEN", legacy_before);
        restore("CONVERGIO_TELEGRAM_CHAT_ID", canonical_chat_before);
        restore("TELEGRAM_CHAT_ID", legacy_chat_before);
    }
}
