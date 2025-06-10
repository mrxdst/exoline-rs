use super::super::controller_loader::LoadMode;
use super::{
    exo_file::ExoFile,
    file_bpac::parse_bpac_file,
    file_internal::{FileInternal, ParseFileError},
    file_vpac::parse_vpac_file,
};

pub fn parse_dpac_file(content: &str, mode: LoadMode, hash: u64) -> Result<FileInternal, ParseFileError> {
    let exo_file = ExoFile::new(content);
    let header = exo_file
        .sections()
        .next()
        .ok_or_else(|| ParseFileError::InvalidSyntax("Missing DPac header section".into()))?;
    match header.name.to_ascii_lowercase().as_str() {
        "vpac" | "qpac" => parse_vpac_file(exo_file, mode, hash),
        "bpac" => parse_bpac_file(exo_file, mode, hash),
        _ => return Err(ParseFileError::InvalidSyntax("Missing DPac header section".into())),
    }
}
