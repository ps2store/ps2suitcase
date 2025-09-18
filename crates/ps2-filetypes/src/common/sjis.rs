use encoding_rs::SHIFT_JIS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SjisEncodeError {
    UnmappableCharacter,
}

pub fn encode_sjis(input: &str) -> Result<Vec<u8>, SjisEncodeError> {
    let (encoded, _, had_errors) = SHIFT_JIS.encode(input);
    if had_errors {
        return Err(SjisEncodeError::UnmappableCharacter);
    }

    Ok(encoded.into_owned())
}

pub fn decode_sjis(input: &[u8]) -> String {
    let (decoded, _, _) = SHIFT_JIS.decode(input);
    decoded.into_owned()
}

pub fn is_roundtrip_sjis(value: &str) -> bool {
    let (encoded, _, encode_errors) = SHIFT_JIS.encode(value);
    if encode_errors {
        return false;
    }

    let (decoded, _, decode_errors) = SHIFT_JIS.decode(&encoded);
    !decode_errors && decoded == value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_decodes_to(input: [u8; 2], expected: &str) {
        assert_eq!(decode_sjis(&input), expected);
    }

    #[test]
    fn decode_crlf_preserves_both_characters() {
        assert_decodes_to([0x0D, 0x0A], "\r\n");
    }

    #[test]
    fn decode_cr_only() {
        assert_decodes_to([0x0D, 0x00], "\r");
    }

    #[test]
    fn decode_lf_only() {
        assert_decodes_to([0x0A, 0x00], "\n");
    }

    #[test]
    fn encode_decode_roundtrip_ascii_punctuation() {
        let input = "SAVE!&LOAD";
        let encoded = encode_sjis(input).expect("encode ASCII punctuation");
        let decoded = decode_sjis(&encoded);

        assert_eq!(decoded, input);
    }

    #[test]
    fn encode_decode_roundtrip_multibyte_japanese() {
        let input = "セーブテスト";
        let encoded = encode_sjis(input).expect("encode Japanese text");
        let decoded = decode_sjis(&encoded);

        assert_eq!(decoded, input);
    }

    #[test]
    fn reports_unmappable_characters() {
        assert!(matches!(
            encode_sjis("𝄞"),
            Err(SjisEncodeError::UnmappableCharacter)
        ));
        assert!(!is_roundtrip_sjis("𝄞"));
        assert!(is_roundtrip_sjis("テスト"));
    }
}
