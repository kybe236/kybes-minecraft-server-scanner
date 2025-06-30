use tokio::{
    io::{self, AsyncReadExt},
    net::TcpStream,
};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[allow(unused)]
pub fn read_var_int_long(var_int: &[u8], offset: Option<&mut usize>) -> i64 {
    read_var_int_generic(var_int, offset, 64)
}

pub fn read_var_int(var_int: &[u8], offset: Option<&mut usize>) -> i32 {
    read_var_int_generic(var_int, offset, 32) as i32
}

fn read_var_int_generic(var_int: &[u8], offset: Option<&mut usize>, max_bits: u32) -> i64 {
    let mut value = 0i64;
    let mut position = 0u32;
    let mut current_offset = offset.as_ref().map_or(0, |ptr| **ptr);

    while current_offset < var_int.len() {
        let byte = var_int[current_offset];
        current_offset += 1;
        value |= i64::from(byte & SEGMENT_BITS) << position;

        if byte & CONTINUE_BIT == 0 {
            break;
        }

        position += 7;
        if position >= max_bits {
            panic!("var_int is too big");
        }
    }

    if let Some(ptr) = offset {
        *ptr = current_offset;
    }

    value
}

#[allow(unused)]
pub fn write_var_long(result: &mut Vec<u8>, value: i64) {
    write_var_int_generic(result, value as u64);
}

pub fn write_var_int(result: &mut Vec<u8>, value: &i32) {
    write_var_int_generic(result, *value as u32 as u64);
}

fn write_var_int_generic(result: &mut Vec<u8>, mut value: u64) {
    loop {
        if value <= SEGMENT_BITS as u64 {
            result.push(value as u8);
            return;
        }
        result.push(((value as u8) & SEGMENT_BITS) | CONTINUE_BIT);
        value >>= 7;
    }
}

pub async fn read_var_int_from_stream(stream: &mut TcpStream) -> io::Result<i32> {
    let mut num_read = 0;
    let mut value = 0u32;

    loop {
        let byte = stream.read_u8().await?;
        value |= u32::from(byte & SEGMENT_BITS) << (7 * num_read);
        num_read += 1;

        if byte & CONTINUE_BIT == 0 {
            break;
        }
    }

    Ok(value as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_varint(value: i32) -> Vec<u8> {
        let mut vec = Vec::new();
        write_var_int(&mut vec, &value);
        vec
    }

    fn encode_varlong(value: i64) -> Vec<u8> {
        let mut vec = Vec::new();
        write_var_long(&mut vec, value);
        vec
    }

    fn decode_varint(bytes: &[u8]) -> i32 {
        read_var_int(bytes, None)
    }

    fn decode_varlong(bytes: &[u8]) -> i64 {
        read_var_int_long(bytes, None)
    }

    #[test]
    fn test_varint_roundtrip() {
        let values = [
            0,
            1,
            2,
            127,
            128,
            255,
            25565,
            2097151,
            2147483647,
            -1,
            -2147483648,
        ];

        for &val in &values {
            let encoded = encode_varint(val);
            let decoded = decode_varint(&encoded);
            assert_eq!(val, decoded, "Failed on value {val}");
        }
    }

    #[test]
    fn test_varlong_roundtrip() {
        let values = [
            0,
            1,
            2,
            127,
            128,
            255,
            2147483647,
            9223372036854775807,
            -1,
            -2147483648,
            -9223372036854775808,
        ];

        for &val in &values {
            let encoded = encode_varlong(val);
            let decoded = decode_varlong(&encoded);
            assert_eq!(val, decoded, "Failed on value {val}");
        }
    }
}
