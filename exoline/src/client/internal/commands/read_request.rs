use super::super::encoding::*;

use super::CommandFileKind;

#[derive(PartialEq, Debug)]
pub struct ReadRequest {
    pub kind: CommandFileKind,
    pub load_number: u8,
    pub offset: u32,
}

impl Encodable for ReadRequest {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.kind.into());
        encoder.write_u8(self.load_number);
        encoder.write_u24(self.offset);
        Ok(())
    }
}

impl Decodable<Self> for ReadRequest {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            kind: decoder.read_u8()?.into(),
            load_number: decoder.read_u8()?,
            offset: decoder.read_u24()?,
        })
    }
}
