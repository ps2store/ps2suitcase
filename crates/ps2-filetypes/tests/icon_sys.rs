use base64::{engine::general_purpose::STANDARD, Engine};
use ps2_filetypes::{color::Color, IconSys};

fn icon_sys_fixture_bytes() -> Vec<u8> {
    let encoded = include_str!("fixtures/icon_sys.b64");
    let encoded: String = encoded.split_whitespace().collect();
    STANDARD.decode(encoded).expect("decode icon.sys fixture")
}

#[test]
fn parses_and_roundtrips_icon_sys_background_colors() {
    let bytes = icon_sys_fixture_bytes();
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

#[test]
fn icon_sys_roundtrips_shift_jis_title() {
    let bytes = icon_sys_fixture_bytes();
    let mut icon_sys = IconSys::new(bytes);
    icon_sys.title = "SAVE!&テスト".to_string();
    icon_sys.linebreak_pos = "SAVE!&".chars().count() as u16;

    let serialized = icon_sys
        .to_bytes()
        .expect("serialize icon.sys with Shift-JIS title");
    let reparsed = IconSys::new(serialized);

    assert_eq!(reparsed.title, "SAVE!&テスト");
    assert_eq!(reparsed.linebreak_pos, icon_sys.linebreak_pos);
}
