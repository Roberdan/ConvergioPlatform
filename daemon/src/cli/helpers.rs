use std::path::PathBuf;

pub fn convergio_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".convergio")
}

pub fn load_or_create_secret() -> Result<Vec<u8>, String> {
    let path = convergio_dir().join("mesh-secret");
    if path.exists() {
        let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return Ok(s.trim().as_bytes().to_vec());
    }
    // Generate a random secret and persist it
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, &hex).map_err(|e| e.to_string())?;
    Ok(hex.as_bytes().to_vec())
}

pub fn default_peers_path() -> PathBuf {
    let candidate = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude/config/peers.conf");
    if candidate.exists() {
        return candidate;
    }
    convergio_dir().join("peers.conf")
}

pub fn json_ok(data: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"success": true, "data": data}))
        .unwrap_or_else(|_| r#"{"success":true}"#.into()));
}

pub fn json_err(msg: &str) -> ! {
    eprintln!("{}", serde_json::to_string_pretty(&serde_json::json!({"success": false, "error": msg}))
        .unwrap_or_else(|_| format!(r#"{{"success":false,"error":{:?}}}"#, msg)));
    std::process::exit(1);
}

pub fn open_token_db() -> rusqlite::Connection {
    use convergiomesh_core::token::init_token_db;
    let db_path = convergio_dir().join("tokens.db");
    std::fs::create_dir_all(db_path.parent().unwrap())
        .unwrap_or_else(|e| json_err(&format!("cannot create ~/.convergio: {e}")));
    let conn = rusqlite::Connection::open(&db_path)
        .unwrap_or_else(|e| json_err(&format!("cannot open tokens.db: {e}")));
    init_token_db(&conn)
        .unwrap_or_else(|e| json_err(&format!("cannot init token db: {e}")));
    conn
}
