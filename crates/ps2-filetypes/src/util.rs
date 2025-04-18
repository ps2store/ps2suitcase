pub fn parse_cstring(input: &[u8]) -> String {
    let mut result = input.to_vec();
    if let Some(first) = result.iter().position(|&b| b == 0) {
        result.truncate(first);
    }
    String::from_utf8_lossy(&result).to_string()
}
