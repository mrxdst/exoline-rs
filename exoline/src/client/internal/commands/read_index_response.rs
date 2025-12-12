use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadIndexResponse {
    pub value: u8,
}

impl Encodable for ReadIndexResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.value);
        Ok(())
    }
}

impl Decodable<Self> for ReadIndexResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self { value: decoder.read_u8()? })
    }
}
