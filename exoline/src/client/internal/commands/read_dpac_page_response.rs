use std::borrow::Cow;

use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadDPacPageResponse<'a> {
    pub data: Cow<'a, [u8]>,
}

impl<'a> Encodable for ReadDPacPageResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_bytes(&self.data);
        return Ok(());
    }
}

impl<'a> Decodable<Self> for ReadDPacPageResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            data: decoder.read_bytes(decoder.remaining())?.into(),
        });
    }
}
