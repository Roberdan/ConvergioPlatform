use image::{DynamicImage, Luma};
use qrcode::render::unicode;
use qrcode::QrCode;

#[derive(Debug)]
pub enum QrError {
    GenerationFailed(String),
    EncodingFailed(String),
}

/// Generate QR code as unicode string (for terminal display)
pub fn generate_qr_terminal(data: &str) -> Result<String, QrError> {
    let code =
        QrCode::new(data.as_bytes()).map_err(|e| QrError::GenerationFailed(e.to_string()))?;
    let string = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    Ok(string)
}

/// Generate QR code as PNG bytes (for GUI display)
pub fn generate_qr_png(data: &str, size: u32) -> Result<Vec<u8>, QrError> {
    let code =
        QrCode::new(data.as_bytes()).map_err(|e| QrError::GenerationFailed(e.to_string()))?;
    let image = code.render::<Luma<u8>>().min_dimensions(size, size).build();
    let dynamic = DynamicImage::ImageLuma8(image);
    let mut bytes = Vec::new();
    dynamic
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| QrError::EncodingFailed(e.to_string()))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_qr_not_empty() {
        let result = generate_qr_terminal("test-token-abc123").unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('\u{2588}') || result.contains('\u{2580}') || result.len() > 50);
    }

    #[test]
    fn test_png_qr_valid() {
        let bytes = generate_qr_png("test-token-abc123", 200).unwrap();
        assert!(!bytes.is_empty());
        // PNG magic bytes
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_empty_data_still_works() {
        let result = generate_qr_terminal("").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_long_token() {
        let long_token = "a".repeat(500);
        let result = generate_qr_terminal(&long_token).unwrap();
        assert!(!result.is_empty());
    }
}
