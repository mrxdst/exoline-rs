use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadHugeResponse {
    pub value: i32,
}

impl Encodable for ReadHugeResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_i32(self.value);
        Ok(())
    }
}

impl Decodable<Self> for ReadHugeResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self { value: decoder.read_i32()? })
    }
}
