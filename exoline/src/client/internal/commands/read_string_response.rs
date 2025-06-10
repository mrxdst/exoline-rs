use std::borrow::Cow;

use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct ReadStringResponse<'a> {
    pub value: Cow<'a, str>,
}

impl<'a> Encodable for ReadStringResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_string(&self.value)?;
        return Ok(());
    }
}

impl<'a> Decodable<Self> for ReadStringResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            value: decoder.read_string()?.into(),
        });
    }
}
