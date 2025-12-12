use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadLogicResponse {
    pub value: bool,
}

impl Encodable for ReadLogicResponse {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.value as u8);
        Ok(())
    }
}

impl Decodable<Self> for ReadLogicResponse {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            value: decoder.read_u8()? != 0,
        })
    }
}
