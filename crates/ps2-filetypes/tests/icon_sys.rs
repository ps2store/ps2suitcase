use base64::{engine::general_purpose::STANDARD, Engine};
use ps2_filetypes::{color::Color, IconSys};

#[test]
fn parses_and_roundtrips_icon_sys_background_colors() {
    let encoded = include_str!("fixtures/icon_sys.b64");
    let encoded: String = encoded.split_whitespace().collect();
    let bytes = STANDARD
        .decode(encoded)
        .expect("decode icon.sys fixture");
    let icon_sys = IconSys::new(bytes.clone());

    let expected = [
        Color::new(255, 0, 0, 255),
        Color::new(0, 255, 0, 255),
        Color::new(0, 0, 255, 255),
        Color::new(128, 128, 128, 128),
    ];

    for (parsed, expected) in icon_sys.background_colors.iter().zip(expected.iter()) {
        let parsed: [u8; 4] = (*parsed).into();
        let expected: [u8; 4] = (*expected).into();
        assert_eq!(parsed, expected);
    }

    let encoded_bytes = icon_sys
        .to_bytes()
        .expect("serialize icon.sys back to bytes");

    assert_eq!(encoded_bytes, bytes);
}
