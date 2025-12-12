use std::borrow::Cow;

use super::super::encoding::*;

use super::CommandFileKind;

#[derive(PartialEq, Debug)]
pub struct WriteStringRequest<'a> {
    pub kind: CommandFileKind,
    pub load_number: u8,
    pub offset: u32,
    pub value: Cow<'a, str>,
}

impl<'a> Encodable for WriteStringRequest<'a> {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.kind.into());
        encoder.write_u8(self.load_number);
        encoder.write_u24(self.offset);
        encoder.write_string(&self.value)?;
        Ok(())
    }
}

impl<'a> Decodable<Self> for WriteStringRequest<'a> {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            kind: decoder.read_u8()?.into(),
            load_number: decoder.read_u8()?,
            offset: decoder.read_u24()?,
            value: decoder.read_string()?.into(),
        })
    }
}
