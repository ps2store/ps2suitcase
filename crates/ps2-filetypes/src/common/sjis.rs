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
    let mut str_out = vec![0u8; input.len()];

    for (i, pair) in input.chunks_exact(2).enumerate() {
        str_out[i] = match pair[0] {
            0x00 => {
                if pair[1] == 0x00 {
                    b'\0'
                } else {
                    b'?'
                }
            }
            0x81 => match pair[1] {
                0x40 => b' ',
                0x46 => b':',
                0x5E => b'/',
                0x69 => b'(',
                0x6A => b')',
                0x6D => b'[',
                0x6E => b']',
                0x6F => b'{',
                0x70 => b'}',
                _ => b'?',
            },
            0x82 => match pair[1] {
                0x4f..=0x7A => pair[1] - 31,
                0x81..=0x98 => pair[1] - 32,
                0x3F => b' ',
                _ => b'?',
            },
            _ => b'?',
        };
    }

    String::from_utf8_lossy(&str_out).to_string()
}
