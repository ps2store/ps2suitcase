use base64::{engine::general_purpose::STANDARD, Engine};
use ps2_filetypes::{color::Color, IconSys};

fn decode_fixture(path: &str) -> Vec<u8> {
    let encoded = match path {
        "fixtures/icon_sys_float.b64" => include_str!("fixtures/icon_sys_float.b64"),
        "fixtures/icon_sys_int.b64" => include_str!("fixtures/icon_sys_int.b64"),
        _ => panic!("unknown fixture path: {}", path),
    };
    let encoded: String = encoded.split_whitespace().collect();
    STANDARD.decode(encoded).expect("decode icon.sys fixture")
}

fn expected_colors() -> [Color; 4] {
    [
        Color::new(255, 0, 0, 255),
        Color::new(0, 255, 0, 255),
        Color::new(0, 0, 255, 255),
        Color::new(128, 128, 128, 128),
    ]
}

#[test]
fn parses_float_encoded_icon_sys_background_colors_and_serializes_to_integers() {
    let float_bytes = decode_fixture("fixtures/icon_sys_float.b64");
    let icon_sys = IconSys::new(float_bytes);

    for (parsed, expected) in icon_sys
        .background_colors
        .iter()
        .zip(expected_colors().iter())
    {
        let parsed: [u8; 4] = (*parsed).into();
        let expected: [u8; 4] = (*expected).into();
        assert_eq!(parsed, expected);
    }

    let encoded_bytes = icon_sys
        .to_bytes()
        .expect("serialize icon.sys back to bytes");

    let int_bytes = decode_fixture("fixtures/icon_sys_int.b64");
    assert_eq!(encoded_bytes, int_bytes);
}

#[test]
fn parses_and_roundtrips_integer_encoded_icon_sys_background_colors() {
    let bytes = decode_fixture("fixtures/icon_sys_int.b64");
    let icon_sys = IconSys::new(bytes.clone());

    for (parsed, expected) in icon_sys
        .background_colors
        .iter()
        .zip(expected_colors().iter())
    {
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
    let bytes = decode_fixture("fixtures/icon_sys_int.b64");
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
