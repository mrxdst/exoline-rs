use std::borrow::Cow;

use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct GetControllerIdResponse<'a> {
    pub id: Cow<'a, str>,
}

impl<'a> Encodable for GetControllerIdResponse<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_string(&self.id)?;
        return Ok(());
    }
}

impl<'a> Decodable<Self> for GetControllerIdResponse<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            id: decoder.read_string()?.into(),
        });
    }
}
