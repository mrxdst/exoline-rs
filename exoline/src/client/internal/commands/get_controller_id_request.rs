use super::super::encoding::*;

#[derive(PartialEq, Debug)]
pub struct GetControllerIdRequest;

impl Encodable for GetControllerIdRequest {
    fn encode(&self, _encoder: &mut Encoder) -> EncodeResult {
        Ok(())
    }
}

impl Decodable<Self> for GetControllerIdRequest {
    fn decode(_decoder: &mut Decoder) -> DecodeResult<Self> {
        Ok(Self {})
    }
}
