use num_enum::{FromPrimitive, IntoPrimitive};

mod get_controller_id_request;
mod get_controller_id_response;
mod read_dpac_page_request;
mod read_dpac_page_response;
mod read_huge_response;
mod read_index_response;
mod read_integer_response;
mod read_logic_response;
mod read_part_attr_header;
mod read_real_response;
mod read_request;
mod read_string_response;
mod write_huge_request;
mod write_index_request;
mod write_integer_request;
mod write_logic_request;
mod write_real_request;
mod write_string_request;

pub use get_controller_id_request::*;
pub use get_controller_id_response::*;
pub use read_dpac_page_request::*;
pub use read_dpac_page_response::*;
pub use read_huge_response::*;
pub use read_index_response::*;
pub use read_integer_response::*;
pub use read_logic_response::*;
pub use read_part_attr_header::*;
pub use read_real_response::*;
pub use read_request::*;
pub use read_string_response::*;
pub use write_huge_request::*;
pub use write_index_request::*;
pub use write_integer_request::*;
pub use write_logic_request::*;
pub use write_real_request::*;
pub use write_string_request::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, IntoPrimitive, FromPrimitive)]
pub enum CommandFileKind {
    VPac = 0x00,
    Task = 0x01,
    BPac = 0x02,
    #[num_enum(catch_all)]
    Unknown(u8),
}
