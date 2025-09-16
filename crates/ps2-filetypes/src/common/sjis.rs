pub fn encode_sjis(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .iter()
        .flat_map(|b| match *b {
            b' ' => [0x80, 0x3F],
            b':' => [0x81, 0x46],
            b'/' => [0x81, 0x5E],
            b'(' => [0x81, 0x69],
            b')' => [0x81, 0x6A],
            b'[' => [0x81, 0x6D],
            b']' => [0x81, 0x6E],
            b'{' => [0x81, 0x6F],
            b'}' => [0x81, 0x70],
            48..=90 => [0x82, *b + 31],
            97..=122 => [0x82, *b + 32],
            _ => [0x00, 0x00],
        })
        .collect::<Vec<_>>()
}

pub fn decode_sjis(input: &[u8]) -> String {
    let mut str_out = Vec::with_capacity(input.len());

    for pair in input.chunks_exact(2) {
        match pair[0] {
            0x00 => {
                if pair[1] == 0x00 {
                    str_out.push(b'\0');
                } else {
                    str_out.push(b'?');
                }
            }
            0x0D => match pair[1] {
                0x0A => str_out.extend_from_slice(b"\r\n"),
                0x00 => str_out.push(b'\r'),
                _ => str_out.push(b'?'),
            },
            0x0A => match pair[1] {
                0x00 => str_out.push(b'\n'),
                _ => str_out.push(b'?'),
            },
            0x81 => match pair[1] {
                0x40 => str_out.push(b' '),
                0x46 => str_out.push(b':'),
                0x5E => str_out.push(b'/'),
                0x69 => str_out.push(b'('),
                0x6A => str_out.push(b')'),
                0x6D => str_out.push(b'['),
                0x6E => str_out.push(b']'),
                0x6F => str_out.push(b'{'),
                0x70 => str_out.push(b'}'),
                _ => str_out.push(b'?'),
            },
            0x82 => match pair[1] {
                0x4f..=0x7A => str_out.push(pair[1] - 31),
                0x81..=0x99 => str_out.push(pair[1] - 32),
                0x3F => str_out.push(b' '),
                _ => str_out.push(b'?'),
            },
            _ => str_out.push(b'?'),
        };
    }

    String::from_utf8_lossy(&str_out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_crlf_preserves_both_characters() {
        let input = [0x0D, 0x0A];
        assert_eq!(decode_sjis(&input), "\r\n");
    }

    #[test]
    fn decode_cr_only() {
        let input = [0x0D, 0x00];
        assert_eq!(decode_sjis(&input), "\r");
    }

    #[test]
    fn decode_lf_only() {
        let input = [0x0A, 0x00];
        assert_eq!(decode_sjis(&input), "\n");
    }
}
