pub fn read_u16(data: &[u8], index: Option<&mut usize>) -> Result<u16, std::io::Error> {
    let current = index.as_ref().map(|v| **v).unwrap_or(0);
    const SIZE: usize = 2;

    if current + SIZE > data.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Not enough bytes to read u16",
        ));
    }

    let value = u16::from_be_bytes([data[current], data[current + 1]]);
    let new_index = current + SIZE;

    if let Some(idx) = index {
        *idx = new_index;
    }

    Ok(value)
}

#[allow(unused)]
pub fn write_u16(buffer: &mut Vec<u8>, number: u16) {
    buffer.extend_from_slice(&number.to_be_bytes());
}
