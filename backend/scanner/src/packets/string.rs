use super::varint::{read_var_int, write_var_int};

pub fn read_string(data: &[u8], index: &mut usize) -> Result<String, std::io::Error> {
    let length = read_var_int(data, Some(index)) as usize;
    let end_pos = *index + length;

    if end_pos > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Attempted to read beyond the buffer",
        ));
    }

    let str_bytes = &data[*index..end_pos];
    *index = end_pos;

    std::str::from_utf8(str_bytes)
        .map(ToString::to_string)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

pub fn write_string(buffer: &mut Vec<u8>, string: &str) {
    let utf16_len = string.chars().map(|c| c.len_utf16()).sum::<usize>();

    if utf16_len > 32767 {
        panic!("String is too long for the Minecraft protocol!");
    }

    write_var_int(buffer, &(utf16_len as i32));
    buffer.extend_from_slice(string.as_bytes());
}
