use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadRealResponse {
    pub value: f32,
}

impl Encodable for ReadRealResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_f32(self.value);
        Ok(())
    }
}

impl Decodable<Self> for ReadRealResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self { value: decoder.read_f32()? })
    }
}
