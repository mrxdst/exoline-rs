use std::{
    borrow::Cow,
    io::{Cursor, Read},
    num::TryFromIntError,
};

use super::consts::*;

#[derive(PartialEq, Debug)]
pub enum EncodeError {
    Overflow,
}

impl From<TryFromIntError> for EncodeError {
    fn from(_: TryFromIntError) -> Self {
        Self::Overflow
    }
}

pub type EncodeResult = Result<(), EncodeError>;

pub trait Encodable {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult;

    #[allow(unused)]
    fn encode_to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        Encoder::encode(self)
    }
}

pub struct Encoder {
    buffer: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(16),
        }
    }

    #[allow(unused)]
    pub fn position(&self) -> usize {
        self.buffer.len()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend(value.to_le_bytes());
    }

    pub fn write_i16(&mut self, value: i16) {
        self.buffer.extend(value.to_le_bytes());
    }

    pub fn write_u24(&mut self, value: u32) {
        self.buffer.extend(&value.to_le_bytes()[0..3]);
    }

    pub fn write_i32(&mut self, value: i32) {
        self.buffer.extend(value.to_le_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.buffer.extend(value.to_le_bytes());
    }

    pub fn write_string(&mut self, value: &str) -> EncodeResult {
        let bytes = oem_cp::encode_string_lossy(value, &oem_cp::code_table::ENCODING_TABLE_CP850);
        if bytes.len() > 127 {
            return Err(EncodeError::Overflow);
        }
        self.write_bytes(&bytes);
        Ok(())
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.buffer.extend(value);
    }

    pub fn write_type<T>(&mut self, value: &T) -> EncodeResult
    where
        T: Encodable + ?Sized,
    {
        value.encode(self)
    }

    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    pub fn encode<T>(value: &T) -> Result<Vec<u8>, EncodeError>
    where
        T: Encodable + ?Sized,
    {
        let mut encoder = Self::new();
        encoder.write_type(value)?;
        Ok(encoder.finish())
    }
}

#[derive(PartialEq, Debug)]
pub enum DecodeError {
    MissingData,
    InvalidData(String),
}

pub type DecodeResult<T> = Result<T, DecodeError>;

pub trait Decodable<T> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<T>;

    fn decode_from_bytes(buffer: &[u8]) -> DecodeResult<T>
    where
        T: Decodable<T>,
    {
        Decoder::decode(buffer)
    }
}

pub struct Decoder<'a> {
    buffer: &'a [u8],
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Decoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(buffer),
        }
    }

    #[allow(unused)]
    pub fn position(&self) -> usize {
        self.cursor.position() as usize
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.cursor.position() as usize
    }

    pub fn read_u8(&mut self) -> DecodeResult<u8> {
        let value = self.read_bytes_const::<1>()?[0];
        Ok(value)
    }

    pub fn read_u16(&mut self) -> DecodeResult<u16> {
        let bytes = self.read_bytes_const()?;
        let value = u16::from_le_bytes(bytes);
        Ok(value)
    }

    pub fn read_i16(&mut self) -> DecodeResult<i16> {
        let bytes = self.read_bytes_const()?;
        let value = i16::from_le_bytes(bytes);
        Ok(value)
    }

    pub fn read_u24(&mut self) -> DecodeResult<u32> {
        let bytes = self.read_bytes_const::<3>()?;
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
        Ok(value)
    }

    pub fn read_i32(&mut self) -> DecodeResult<i32> {
        let bytes = self.read_bytes_const()?;
        let value = i32::from_le_bytes(bytes);
        Ok(value)
    }

    pub fn read_f32(&mut self) -> DecodeResult<f32> {
        let bytes = self.read_bytes_const()?;
        let value = f32::from_le_bytes(bytes);
        Ok(value)
    }

    pub fn read_string(&mut self) -> DecodeResult<String> {
        let len = usize::min(self.remaining(), 127);
        let bytes = self.read_bytes(len)?;
        let text = oem_cp::decode_string_complete_table(bytes, &oem_cp::code_table::DECODING_TABLE_CP850);
        Ok(text)
    }

    pub fn read_bytes_const<const N: usize>(&mut self) -> DecodeResult<[u8; N]> {
        let mut bytes = [0u8; N];
        self.cursor.read_exact(&mut bytes).map_err(|_| DecodeError::MissingData)?;
        Ok(bytes)
    }

    pub fn read_bytes(&mut self, length: usize) -> DecodeResult<Vec<u8>> {
        let mut bytes = vec![0u8; length];
        self.cursor.read_exact(&mut bytes).map_err(|_| DecodeError::MissingData)?;
        Ok(bytes)
    }

    pub fn read_type<T>(&mut self) -> DecodeResult<T>
    where
        T: Decodable<T>,
    {
        T::decode(self)
    }

    pub fn decode<T>(buffer: &'a [u8]) -> DecodeResult<T>
    where
        T: Decodable<T>,
    {
        let mut decoder = Self::new(buffer);
        let value: T = decoder.read_type()?;
        Ok(value)
    }
}

fn calc_crc(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for byte in data {
        crc ^= byte;
    }
    crc
}

pub fn escape(data: &[u8]) -> Cow<'_, [u8]> {
    let mut i = 0;
    let mut do_escape = false;

    while i < data.len() {
        let byte = data[i];
        match byte {
            ESCAPE_VALUE | BEGIN_REQUEST | BEGIN_RESPONSE | END_MESSAGE => {
                do_escape = true;
                break;
            }
            _ => {}
        }
        i += 1;
    }

    if !do_escape {
        return data.into();
    }

    let mut new_data = Vec::with_capacity(data.len() + 5); // at least +1
    new_data.extend_from_slice(&data[0..i]);

    for byte in &data[i..] {
        match *byte {
            ESCAPE_VALUE | BEGIN_REQUEST | BEGIN_RESPONSE | END_MESSAGE => {
                new_data.push(0x1B);
                new_data.push(!byte);
            }
            _ => new_data.push(*byte),
        }
    }

    new_data.into()
}

pub fn unescape(data: &[u8]) -> Cow<'_, [u8]> {
    let mut i = 0;
    let mut do_unescape = false;

    while i < data.len() {
        let byte = data[i];
        if byte == ESCAPE_VALUE {
            do_unescape = true;
            break;
        }
        i += 1;
    }

    if !do_unescape {
        return data.into();
    }

    let mut new_data = Vec::with_capacity(data.len().saturating_sub(1));
    new_data.extend_from_slice(&data[0..i]);

    let mut unescape_next = false;

    for byte in &data[i..] {
        if unescape_next {
            new_data.push(!byte);
            unescape_next = false;
            continue;
        }
        match *byte {
            ESCAPE_VALUE => unescape_next = true,
            _ => new_data.push(*byte),
        }
    }

    new_data.into()
}

pub fn append_crc(data: &mut Vec<u8>) {
    let crc = calc_crc(data);
    data.push(crc);
}

pub fn verify_and_remove_crc(data: &[u8]) -> Option<&[u8]> {
    if data.is_empty() {
        return None;
    }
    let crc = data[data.len() - 1];
    let body = &data[0..data.len() - 1];
    if crc != calc_crc(body) {
        return None;
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode() {
        let mut encoder = Encoder::new();
        encoder.write_u8(0xAA);
        encoder.write_i16(0x7BCC);
        encoder.write_bytes(&[1, 2, 3]);
        encoder.write_u24(0x12345678);

        assert_eq!(encoder.position(), 9);

        let bytes = encoder.finish();

        assert_eq!(bytes.len(), 9);

        let mut decoder = Decoder::new(&bytes);

        assert_eq!(decoder.position(), 0);
        assert_eq!(decoder.remaining(), 9);

        assert_eq!(decoder.read_u8(), Ok(0xAA));
        assert_eq!(decoder.read_i16(), Ok(0x7BCC));
        assert_eq!(decoder.read_bytes(3), Ok(vec![1, 2, 3]));
        assert_eq!(decoder.read_u24(), Ok(0x00345678));

        assert_eq!(decoder.position(), 9);
        assert_eq!(decoder.remaining(), 0);
    }
}
