pub mod sync;
pub mod validate;

pub use sync::{
    check_token_sync_health, delete_token, get_token, list_tokens, local_host, revoke_token,
    rotate_keys, store_token, sync_tokens_from_peer, TokenInfo, TokenSyncHealth,
};
pub use validate::{
    credential_watch_paths, decrypt_token, derive_key, encrypt_token, watch_credential_files,
    CredentialChange,
};

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE ipc_auth_tokens (id INTEGER PRIMARY KEY, service TEXT NOT NULL, encrypted_token BLOB NOT NULL, nonce BLOB NOT NULL, host TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT '', UNIQUE (service, host));").unwrap();
        conn
    }

    #[test]
    fn test_derive_key_consistent() {
        let k1 = derive_key("secret123");
        let k2 = derive_key("secret123");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_derive_key_different_secrets() {
        let k1 = derive_key("secret1");
        let k2 = derive_key("secret2");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key("test-secret");
        let (ct, nonce) = encrypt_token(&key, "my-token-value");
        let pt = decrypt_token(&key, &ct, &nonce).unwrap();
        assert_eq!(pt, "my-token-value");
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = derive_key("right-key");
        let key2 = derive_key("wrong-key");
        let (ct, nonce) = encrypt_token(&key1, "secret");
        assert!(decrypt_token(&key2, &ct, &nonce).is_err());
    }

    #[test]
    fn test_store_get_roundtrip() {
        let conn = setup_db();
        store_token(&conn, "claude", "tok-123", "secret").unwrap();
        let val = get_token(&conn, "claude", "secret").unwrap();
        assert_eq!(val, Some("tok-123".to_string()));
    }

    #[test]
    fn test_revoke_token() {
        let conn = setup_db();
        store_token(&conn, "gh", "ghp_abc", "s").unwrap();
        let host = local_host();
        let n = revoke_token(&conn, "gh", &host).unwrap();
        assert_eq!(n, 1);
        assert_eq!(get_token(&conn, "gh", "s").unwrap(), None);
    }

    #[test]
    fn test_rotate_keys() {
        let conn = setup_db();
        store_token(&conn, "svc", "val", "old").unwrap();
        let n = rotate_keys(&conn, "old", "new").unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            get_token(&conn, "svc", "new").unwrap(),
            Some("val".to_string())
        );
        assert_eq!(get_token(&conn, "svc", "old").unwrap(), None);
    }

    #[test]
    fn test_list_tokens() {
        let conn = setup_db();
        store_token(&conn, "a", "t1", "s").unwrap();
        store_token(&conn, "b", "t2", "s").unwrap();
        let list = list_tokens(&conn).unwrap();
        assert_eq!(list.len(), 2);
    }
}
