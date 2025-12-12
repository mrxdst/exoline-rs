use std::collections::HashMap;
use std::{collections::VecDeque, error::Error, fmt::Display, sync::Arc};

use tokio::{net::TcpStream, task::AbortHandle};
use tokio::{
    sync::{oneshot, Mutex},
    task::JoinHandle,
};

use crate::controller::{File, FileKind, Variable, VariableKind};

use super::internal::{command_id::CommandId, commands::*, connection::*, encoding::*};
use super::{exoline_exception::EXOlineException, variant::Variant};

/// Errors returned by the [`EXOlineTCPClient`].
#[derive(Debug, Clone)]
pub enum EXOlineError {
    /// Represent an IO error.
    IO(Arc<std::io::Error>),
    /// Some arguments provided to the function are invalid or out of range.
    /// The request was never sent to the server.
    InvalidArguments(String),
    /// Indicates that the response received from the server is not a valid response.
    InvalidResponse(String),
    /// Exception code reported by the server.
    ExolineException(EXOlineException),
}

impl Display for EXOlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(err) => write!(f, "{err}"),
            Self::InvalidArguments(err) => write!(f, "Argument out of range: {err}"),
            Self::InvalidResponse(err) => write!(f, "Invalid response: {err}"),
            Self::ExolineException(ex) => write!(f, "{ex:?}"),
        }
    }
}

impl Error for EXOlineError {}

impl From<DecodeError> for EXOlineError {
    fn from(_: DecodeError) -> Self {
        EXOlineError::InvalidResponse("Error when decoding response".into())
    }
}

type ResponseResult = Result<Vec<u8>, EXOlineError>;
type ResponseQueue = Arc<Mutex<VecDeque<oneshot::Sender<ResponseResult>>>>;

/// EXOline TCP client. Supports reading and writing to a device.
pub struct EXOlineTCPClient {
    connection: Arc<Connection>,
    response_queue: ResponseQueue,
    abort_handle: AbortHandle,
}

impl EXOlineTCPClient {
    pub fn new(stream: TcpStream) -> (Self, JoinHandle<Result<(), EXOlineError>>) {
        let connection = Arc::new(Connection::new(stream));
        let response_queue = Arc::new(Mutex::new(VecDeque::new()));

        let join_handle = tokio::spawn(Self::receive_response(connection.clone(), response_queue.clone()));

        let client = Self {
            connection,
            response_queue,
            abort_handle: join_handle.abort_handle(),
        };

        (client, join_handle)
    }

    async fn receive_response(connection: Arc<Connection>, response_queue: ResponseQueue) -> Result<(), EXOlineError> {
        loop {
            let msg = match connection.read_response().await {
                Ok(Some(msg)) => msg,
                Ok(None) => return Ok(()),
                Err(error) => {
                    let error = match error {
                        ReadError::IO(error) => EXOlineError::IO(error.into()),
                        ReadError::InvalidData => EXOlineError::InvalidResponse("The server sent invalid data".into()),
                    };
                    let mut response_queue = response_queue.lock().await;
                    while let Some(sender) = response_queue.pop_front() {
                        _ = sender.send(Err(error.clone()));
                    }
                    return Err(error);
                }
            };

            let sender = response_queue.lock().await.pop_front();
            match sender {
                None => return Err(EXOlineError::InvalidResponse("The server sent an unexpected response".into())),
                Some(sender) => _ = sender.send(Ok(msg)),
            }
        }
    }
}

impl EXOlineTCPClient {
    /// Reads the EXOline address of the connected controller.
    pub async fn read_exoline_address(&self) -> Result<(u8, u8), EXOlineError> {
        let pla = self.read_variable_raw((255, 30), FileKind::VPac, 0xF1, VariableKind::Index, 0).await?;
        let ela = self.read_variable_raw((255, 30), FileKind::VPac, 0xF1, VariableKind::Index, 1).await?;
        let address = (pla.index().unwrap(), ela.index().unwrap());
        Ok(address)
    }

    /// Reads a page from a DPac. Strings are not read.
    pub async fn read_dpac_page(&self, address: (u8, u8), file: &File, page: u8) -> Result<HashMap<Variable, Variant>, EXOlineError> {
        self.read_dpac_internal(address, file, Some(page)).await
    }

    /// Reads an entire DPac. Strings are not read.
    pub async fn read_dpac(&self, address: (u8, u8), file: &File) -> Result<HashMap<Variable, Variant>, EXOlineError> {
        self.read_dpac_internal(address, file, None).await
    }

    async fn read_dpac_internal(&self, address: (u8, u8), file: &File, only_page: Option<u8>) -> Result<HashMap<Variable, Variant>, EXOlineError> {
        match file.kind() {
            FileKind::BPac | FileKind::VPac => {}
            _ => return Err(EXOlineError::InvalidArguments("Can only read pages from DPac's".into())),
        }

        let mut result = HashMap::with_capacity(only_page.map(|_| 60).unwrap_or_else(|| file.len()));

        let mut data = Vec::new();
        let mut page: i32 = -1;

        for variable in file.iter() {
            if variable.kind() == VariableKind::String {
                continue;
            }

            let file_offset = variable.offset() as usize;

            let (page_size, page_offset) = match file.kind() {
                FileKind::BPac => (variable.kind().page_size_of_bpac_variable() as usize, file_offset),
                FileKind::VPac => (variable.kind().page_size_of_vpac_variable() as usize, file_offset * 2),
                _ => unreachable!(),
            };

            if let Some(only_page) = only_page {
                if page_offset / 120 != only_page as usize {
                    continue;
                }
            }

            let mut bytes = None;
            loop {
                let data_offset = match only_page {
                    None => page_offset,
                    Some(_) => page_offset % 120,
                };
                match data.get(data_offset..data_offset + page_size).map(|b| b.to_owned()) {
                    Some(b) => {
                        bytes = Some(b);
                        break;
                    }
                    None => {
                        if page >= 0xFF {
                            break;
                        }
                        page += 1;
                        match self
                            .read_dpac_page_raw(address, file.kind(), file.load_number(), only_page.unwrap_or(page as u8))
                            .await
                        {
                            Ok(mut next_data) => {
                                next_data.resize(120, 0); // in case
                                data.extend(next_data);
                                if only_page.is_some() {
                                    page = 0xFF;
                                }
                            }
                            Err(EXOlineError::ExolineException(EXOlineException::AddressOutsideRange)) => {
                                page = 0xFF; // No more pages.
                                break;
                            }
                            Err(err) => return Err(err),
                        }
                    }
                }
            }

            let bytes = match bytes {
                None => continue,
                Some(bytes) => bytes,
            };

            let variant = match file.kind() {
                FileKind::VPac => match variable.kind() {
                    VariableKind::Huge => Variant::Huge(i32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]])),
                    VariableKind::Index => Variant::Index(bytes[1]),
                    VariableKind::Integer => Variant::Integer(i16::from_le_bytes([bytes[1], bytes[2]])),
                    VariableKind::Logic => Variant::Logic(bytes[1] != 0),
                    VariableKind::Real => Variant::Real(f32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]])),
                    VariableKind::String => unreachable!(),
                },
                FileKind::BPac => match variable.kind() {
                    VariableKind::Huge => Variant::Huge(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
                    VariableKind::Index => Variant::Index(bytes[0]),
                    VariableKind::Integer => Variant::Integer(i16::from_le_bytes([bytes[0], bytes[1]])),
                    VariableKind::Logic => Variant::Logic(bytes[0] != 0),
                    VariableKind::Real => Variant::Real(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
                    VariableKind::String => unreachable!(),
                },
                _ => unreachable!(),
            };

            result.insert(variable, variant);
        }

        Ok(result)
    }

    /// Read a page from a DPac by manually providing the parameters
    pub async fn read_dpac_page_raw(&self, address: (u8, u8), file_kind: FileKind, load_number: u8, page: u8) -> Result<Vec<u8>, EXOlineError> {
        match file_kind {
            FileKind::BPac | FileKind::VPac => {}
            _ => {
                return Err(EXOlineError::InvalidArguments("Can only read pages from DPac's".into()));
            }
        }
        let response_data = self
            .send_request(address, CommandId::ReadDPacPage, &ReadDPacPageRequest { load_number, page })
            .await?;
        let response = ReadDPacPageResponse::decode_from_bytes(&response_data)?;
        Ok(response.data.into())
    }

    /// Read an entire DPac by manually providing the parameters
    pub async fn read_dpac_raw(&self, address: (u8, u8), file_kind: FileKind, load_number: u8) -> Result<Vec<u8>, EXOlineError> {
        let mut data = Vec::new();

        for page in 0..=0xFF {
            match self.read_dpac_page_raw(address, file_kind, load_number, page).await {
                Ok(bytes) => data.extend(bytes),
                Err(EXOlineError::ExolineException(EXOlineException::AddressOutsideRange)) => break,
                Err(err) => return Err(err),
            }
        }

        Ok(data)
    }

    /// Read a variable
    pub async fn read_variable(&self, address: (u8, u8), variable: &Variable) -> Result<Variant, EXOlineError> {
        self.read_variable_raw(address, variable.file_kind(), variable.load_number(), variable.kind(), variable.offset())
            .await
    }

    /// Read a variable by manually providing the parameters
    pub async fn read_variable_raw(
        &self,
        address: (u8, u8),
        file_kind: FileKind,
        load_number: u8,
        variable_kind: VariableKind,
        offset: u32,
    ) -> Result<Variant, EXOlineError> {
        match file_kind {
            FileKind::Task => {
                let request = ReadRequest {
                    kind: CommandFileKind::Task,
                    load_number,
                    offset,
                };
                match variable_kind {
                    VariableKind::Huge => {
                        let response_data = self.send_request(address, CommandId::ReadHuge, &request).await?;
                        let response = ReadHugeResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Huge(response.value))
                    }
                    VariableKind::Index => {
                        let response_data = self.send_request(address, CommandId::ReadIndex, &request).await?;
                        let response = ReadIndexResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Index(response.value))
                    }
                    VariableKind::Integer => {
                        let response_data = self.send_request(address, CommandId::ReadInteger, &request).await?;
                        let response = ReadIntegerResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Integer(response.value))
                    }
                    VariableKind::Logic => {
                        let response_data = self.send_request(address, CommandId::ReadLogic, &request).await?;
                        let response = ReadLogicResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Logic(response.value))
                    }
                    VariableKind::Real => {
                        let response_data = self.send_request(address, CommandId::ReadReal, &request).await?;
                        let response = ReadRealResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Real(response.value))
                    }
                    VariableKind::String => {
                        let response_data = self.send_request(address, CommandId::ReadString, &request).await?;
                        let response = ReadStringResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::String(response.value.to_string()))
                    }
                }
            }
            FileKind::VPac => {
                let request = ReadRequest {
                    kind: CommandFileKind::VPac,
                    load_number,
                    offset,
                };
                match variable_kind {
                    VariableKind::Huge => {
                        let response_data = self.send_request(address, CommandId::ReadHuge, &request).await?;
                        let response = ReadHugeResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Huge(response.value))
                    }
                    VariableKind::Index => {
                        let response_data = self.send_request(address, CommandId::ReadIndex, &request).await?;
                        let response = ReadIndexResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Index(response.value))
                    }
                    VariableKind::Integer => {
                        let response_data = self.send_request(address, CommandId::ReadInteger, &request).await?;
                        let response = ReadIntegerResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Integer(response.value))
                    }
                    VariableKind::Logic => {
                        let response_data = self.send_request(address, CommandId::ReadLogic, &request).await?;
                        let response = ReadLogicResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Logic(response.value))
                    }
                    VariableKind::Real => {
                        let response_data = self.send_request(address, CommandId::ReadReal, &request).await?;
                        let response = ReadRealResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Real(response.value))
                    }
                    VariableKind::String => {
                        let response_data = self.send_request(address, CommandId::ReadString, &request).await?;
                        let response = ReadStringResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::String(response.value.to_string()))
                    }
                }
            }
            FileKind::BPac => {
                let request = ReadRequest {
                    kind: CommandFileKind::BPac,
                    load_number,
                    offset,
                };
                match variable_kind {
                    VariableKind::Huge => {
                        let response_data = self.send_request(address, CommandId::ReadHuge, &request).await?;
                        let response = ReadHugeResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Huge(response.value))
                    }
                    VariableKind::Index => {
                        let response_data = self.send_request(address, CommandId::ReadIndex, &request).await?;
                        let response = ReadIndexResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Index(response.value))
                    }
                    VariableKind::Integer => {
                        let response_data = self.send_request(address, CommandId::ReadInteger, &request).await?;
                        let response = ReadIntegerResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Integer(response.value))
                    }
                    VariableKind::Logic => {
                        let response_data = self.send_request(address, CommandId::ReadLogic, &request).await?;
                        let response = ReadLogicResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Logic(response.value))
                    }
                    VariableKind::Real => {
                        let response_data = self.send_request(address, CommandId::ReadReal, &request).await?;
                        let response = ReadRealResponse::decode_from_bytes(&response_data)?;
                        Ok(Variant::Real(response.value))
                    }
                    VariableKind::String => {
                        Err(EXOlineError::InvalidArguments("Can't read a string from a BPac".into()))
                    }
                }
            }
            FileKind::Text => match variable_kind {
                VariableKind::String => {
                    let request = ReadRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                    };
                    let response_data = self.send_request(address, CommandId::ReadString, &request).await?;
                    let response = ReadStringResponse::decode_from_bytes(&response_data)?;
                    Ok(Variant::String(response.value.to_string()))
                }
                _ => {
                    Err(EXOlineError::InvalidArguments("Can only read strings from text files".into()))
                }
            },
        }
    }

    /// Write a variable
    pub async fn write_variable(&self, address: (u8, u8), variable: &Variable, value: &Variant) -> Result<(), EXOlineError> {
        self.write_variable_raw(
            address,
            variable.file_kind(),
            variable.load_number(),
            variable.kind(),
            variable.offset(),
            value,
        )
        .await
    }

    /// Write a variable by manually providing the parameters
    pub async fn write_variable_raw(
        &self,
        address: (u8, u8),
        file_kind: FileKind,
        load_number: u8,
        variable_kind: VariableKind,
        offset: u32,
        value: &Variant,
    ) -> Result<(), EXOlineError> {
        match file_kind {
            FileKind::Task => match variable_kind {
                VariableKind::Huge => {
                    let Some(value) = value.huge() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteHugeRequest {
                        kind: CommandFileKind::Task,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteHuge, &request).await?;
                    Ok(())
                }
                VariableKind::Index => {
                    let Some(value) = value.index() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteIndexRequest {
                        kind: CommandFileKind::Task,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteIndex, &request).await?;
                    Ok(())
                }
                VariableKind::Integer => {
                    let Some(value) = value.integer() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteIntegerRequest {
                        kind: CommandFileKind::Task,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteInteger, &request).await?;
                    Ok(())
                }
                VariableKind::Logic => {
                    let Some(value) = value.logic() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteLogicRequest {
                        kind: CommandFileKind::Task,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteLogic, &request).await?;
                    Ok(())
                }
                VariableKind::Real => {
                    let Some(value) = value.real() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteRealRequest {
                        kind: CommandFileKind::Task,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteReal, &request).await?;
                    Ok(())
                }
                VariableKind::String => {
                    let Some(value) = value.string() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteStringRequest {
                        kind: CommandFileKind::Task,
                        load_number,
                        offset,
                        value: value.into(),
                    };
                    self.send_request(address, CommandId::WriteString, &request).await?;
                    Ok(())
                }
            },
            FileKind::VPac => match variable_kind {
                VariableKind::Huge => {
                    let Some(value) = value.huge() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteHugeRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteHuge, &request).await?;
                    Ok(())
                }
                VariableKind::Index => {
                    let Some(value) = value.index() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteIndexRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteIndex, &request).await?;
                    Ok(())
                }
                VariableKind::Integer => {
                    let Some(value) = value.integer() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteIntegerRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteInteger, &request).await?;
                    Ok(())
                }
                VariableKind::Logic => {
                    let Some(value) = value.logic() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteLogicRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteLogic, &request).await?;
                    Ok(())
                }
                VariableKind::Real => {
                    let Some(value) = value.real() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteRealRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteReal, &request).await?;
                    Ok(())
                }
                VariableKind::String => {
                    let Some(value) = value.string() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteStringRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value: value.into(),
                    };
                    self.send_request(address, CommandId::WriteString, &request).await?;
                    Ok(())
                }
            },
            FileKind::BPac => match variable_kind {
                VariableKind::Huge => {
                    let Some(value) = value.huge() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteHugeRequest {
                        kind: CommandFileKind::BPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteHuge, &request).await?;
                    Ok(())
                }
                VariableKind::Index => {
                    let Some(value) = value.index() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteIndexRequest {
                        kind: CommandFileKind::BPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteIndex, &request).await?;
                    Ok(())
                }
                VariableKind::Integer => {
                    let Some(value) = value.integer() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteIntegerRequest {
                        kind: CommandFileKind::BPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteInteger, &request).await?;
                    Ok(())
                }
                VariableKind::Logic => {
                    let Some(value) = value.logic() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteLogicRequest {
                        kind: CommandFileKind::BPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteLogic, &request).await?;
                    Ok(())
                }
                VariableKind::Real => {
                    let Some(value) = value.real() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteRealRequest {
                        kind: CommandFileKind::BPac,
                        load_number,
                        offset,
                        value,
                    };
                    self.send_request(address, CommandId::WriteReal, &request).await?;
                    Ok(())
                }
                VariableKind::String => {
                    Err(EXOlineError::InvalidArguments("Can't write a string to a BPac".into()))
                }
            },
            FileKind::Text => match variable_kind {
                VariableKind::String => {
                    let Some(value) = value.string() else {
                        return Err(EXOlineError::InvalidArguments("The variable and value kind doesn't match".into()));
                    };
                    let request = WriteStringRequest {
                        kind: CommandFileKind::VPac,
                        load_number,
                        offset,
                        value: value.into(),
                    };
                    self.send_request(address, CommandId::ReadString, &request).await?;
                    Ok(())
                }
                _ => {
                    Err(EXOlineError::InvalidArguments("Can only write strings to text files".into()))
                }
            },
        }
    }

    /// Reads the controller model and version as a string
    pub async fn read_controller_id(&self, address: (u8, u8)) -> Result<String, EXOlineError> {
        let response_data = self.send_request(address, CommandId::GetControllerId, &GetControllerIdRequest).await?;
        let response = GetControllerIdResponse::decode_from_bytes(&response_data)?;
        Ok(response.id.into())
    }

    /// Read a partition attribute by manually providing the parameters
    pub async fn read_partition_attribute(
        &self,
        address: (u8, u8),
        partition: u8,
        attribute_kind: VariableKind,
        attribute_id: u16,
    ) -> Result<Variant, EXOlineError> {
        match attribute_kind {
            VariableKind::Huge => {
                let request = ReadPartAttrHeader {
                    kind: PartAttrHeaderKind::Huge,
                    part_no: partition,
                    attr: attribute_id,
                };
                let response_data = self.send_request(address, CommandId::ReadPartAttrHeader, &request).await?;
                let response = ReadHugeResponse::decode_from_bytes(&response_data)?;
                Ok(Variant::Huge(response.value))
            }
            VariableKind::Real => {
                let request = ReadPartAttrHeader {
                    kind: PartAttrHeaderKind::Real,
                    part_no: partition,
                    attr: attribute_id,
                };
                let response_data = self.send_request(address, CommandId::ReadPartAttrHeader, &request).await?;
                let response = ReadRealResponse::decode_from_bytes(&response_data)?;
                Ok(Variant::Real(response.value))
            }
            VariableKind::String => {
                let request = ReadPartAttrHeader {
                    kind: PartAttrHeaderKind::String,
                    part_no: partition,
                    attr: attribute_id,
                };
                let response_data = self.send_request(address, CommandId::ReadPartAttrHeader, &request).await?;
                let response = ReadStringResponse::decode_from_bytes(&response_data)?;
                Ok(Variant::String(response.value.to_string()))
            }
            kind => {
                Err(EXOlineError::InvalidArguments(format!("Can't read a {:?} from a partition header", kind)))
            }
        }
    }

    async fn send_request<T>(&self, address: (u8, u8), command_id: CommandId, request: &T) -> Result<Vec<u8>, EXOlineError>
    where
        T: Encodable,
    {
        let mut encoder = Encoder::new();
        encoder.write_u8(address.0);
        encoder.write_u8(address.1);
        encoder.write_u8(command_id.into());
        encoder
            .write_type(request)
            .map_err(|_| EXOlineError::InvalidArguments("Error encoding message".into()))?;

        let mut request_data = encoder.finish();
        append_crc(&mut request_data);
        let request_data = escape(&request_data);

        let (sender, receiver) = oneshot::channel::<ResponseResult>();

        {
            let mut map = self.response_queue.lock().await;
            map.push_back(sender);
        }

        match self.connection.write_request(&request_data).await {
            Ok(_) => {}
            Err(e) => return Err(EXOlineError::IO(e.into())),
        }

        let response_data = match receiver.await.unwrap() {
            Ok(data) => data,
            Err(error) => return Err(error),
        };

        if response_data.len() == 1 {
            return Err(EXOlineError::ExolineException(response_data[0].into()));
        }

        let response_data = unescape(&response_data);
        let response_data = verify_and_remove_crc(&response_data).ok_or_else(|| EXOlineError::InvalidResponse("CRC mismatch".into()))?;

        Ok(response_data.into())
    }
}

impl Drop for EXOlineTCPClient {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}
