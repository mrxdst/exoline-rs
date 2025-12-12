use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

use super::consts::*;

pub struct Connection {
    reader: Mutex<BufReader<OwnedReadHalf>>,
    writer: Mutex<BufWriter<OwnedWriteHalf>>,
}

#[derive(Debug)]
pub enum ReadError {
    IO(std::io::Error),
    InvalidData,
}

impl From<std::io::Error> for ReadError {
    fn from(value: std::io::Error) -> Self {
        ReadError::IO(value)
    }
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            reader: Mutex::new(BufReader::new(reader)),
            writer: Mutex::new(BufWriter::new(writer)),
        }
    }

    pub async fn read_response(&self) -> Result<Option<Vec<u8>>, ReadError> {
        let mut reader = self.reader.lock().await;
        let mut buffer: Option<Vec<u8>> = None;
        loop {
            let mut byte = [0; 1];

            let bytes_read = reader.read(&mut byte).await?;

            if bytes_read == 0 {
                _ = self.writer.lock().await.shutdown().await;
                return Ok(None);
            }

            match byte[0] {
                BEGIN_RESPONSE => match buffer {
                    Some(_) => {
                        return Err(ReadError::InvalidData);
                    }
                    None => buffer = Some(Vec::with_capacity(16)),
                },
                BEGIN_REQUEST => {
                    return Err(ReadError::InvalidData);
                }
                END_MESSAGE => match buffer.take() {
                    None => {
                        return Err(ReadError::InvalidData);
                    }
                    Some(buffer) => {
                        return Ok(Some(buffer));
                    }
                },
                value => match &mut buffer {
                    None => {
                        return Err(ReadError::InvalidData);
                    }
                    Some(buffer) => {
                        if buffer.len() > 1024 {
                            return Err(ReadError::InvalidData);
                        }
                        buffer.push(value);
                    }
                },
            }
        }
    }

    pub async fn write_request(&self, data: &[u8]) -> Result<(), std::io::Error> {
        let mut writer = self.writer.lock().await;

        writer.write_u8(BEGIN_REQUEST).await?;
        writer.write_all(data).await?;
        writer.write_u8(END_MESSAGE).await?;
        writer.flush().await?;

        Ok(())
    }
}
