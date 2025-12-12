use num_enum::{FromPrimitive, IntoPrimitive};

use super::super::encoding::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, IntoPrimitive, FromPrimitive)]
pub enum PartAttrHeaderKind {
    Huge = 0x10,
    Real = 0x12,
    String = 0x14,
    #[num_enum(catch_all)]
    Unknown(u8),
}

#[derive(PartialEq, Debug)]
pub struct ReadPartAttrHeader {
    pub kind: PartAttrHeaderKind,
    pub part_no: u8,
    pub attr: u16,
}

impl Encodable for ReadPartAttrHeader {
    fn encode(&self, encoder: &mut Encoder) -> EncodeResult {
        encoder.write_u8(self.kind.into());
        encoder.write_u8(self.part_no);
        encoder.write_u16(self.attr);
        Ok(())
    }
}

impl Decodable<Self> for ReadPartAttrHeader {
    fn decode(decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {
            kind: decoder.read_u8()?.into(),
            part_no: decoder.read_u8()?,
            attr: decoder.read_u16()?,
        })
    }
}
