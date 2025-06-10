use super::super::encoding::*;

use super::CommandFileKind;

#[derive(PartialEq, Debug)]
pub struct WriteIndexRequest {
    pub kind: CommandFileKind,
    pub load_number: u8,
    pub offset: u32,
    pub value: u8,
}

impl Encodable for WriteIndexRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.kind.into());
        encoder.write_u8(self.load_number);
        encoder.write_u24(self.offset);
        encoder.write_u8(self.value);
        return Ok(());
    }
}

impl Decodable<Self> for WriteIndexRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        return Ok(Self {
            kind: decoder.read_u8()?.into(),
            load_number: decoder.read_u8()?,
            offset: decoder.read_u24()?,
            value: decoder.read_u8()?,
        });
    }
}
