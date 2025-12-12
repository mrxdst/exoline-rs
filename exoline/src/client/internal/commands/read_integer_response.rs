use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadIntegerResponse {
    pub value: i16,
}

impl Encodable for ReadIntegerResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_i16(self.value);
        Ok(())
    }
}

impl Decodable<Self> for ReadIntegerResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self { value: decoder.read_i16()? })
    }
}
