use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadDPacPageRequest {
    pub load_number: u8,
    pub page: u8,
}

impl Encodable for ReadDPacPageRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.load_number);
        encoder.write_u8(self.page);
        Ok(())
    }
}

impl Decodable<Self> for ReadDPacPageRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            load_number: decoder.read_u8()?,
            page: decoder.read_u8()?,
        })
    }
}
