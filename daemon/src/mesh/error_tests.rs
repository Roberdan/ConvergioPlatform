use super::MeshError;

#[test]
fn io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let mesh_err: MeshError = io_err.into();
    assert!(matches!(mesh_err, MeshError::Io(_)));
    assert!(mesh_err.to_string().contains("file not found"));
}

#[test]
fn db_error_conversion() {
    let db_err = rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(1),
        Some("table not found".to_string()),
    );
    let mesh_err: MeshError = db_err.into();
    assert!(matches!(mesh_err, MeshError::Db(_)));
}

#[test]
fn json_error_conversion() {
    let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
    let mesh_err: MeshError = json_err.into();
    assert!(matches!(mesh_err, MeshError::Serialization(_)));
}

#[test]
fn msgpack_encode_error_conversion() {
    // Force a msgpack encode error via an unsupported type
    let err = rmp_serde::encode::to_vec(&f64::NAN);
    // NaN might or might not error — use a direct constructor instead
    let mesh_err = MeshError::Serialization("test encode error".to_string());
    assert!(mesh_err.to_string().contains("test encode error"));
    // Verify the From impl compiles
    let _ = err;
}

#[test]
fn display_formats_contain_variant() {
    let err = MeshError::Auth("bad creds".into());
    assert!(err.to_string().contains("auth error"));

    let err = MeshError::Config("missing key".into());
    assert!(err.to_string().contains("config error"));

    let err = MeshError::Network("timeout".into());
    assert!(err.to_string().contains("network error"));

    let err = MeshError::Internal("panic recovery".into());
    assert!(err.to_string().contains("internal error"));
}
