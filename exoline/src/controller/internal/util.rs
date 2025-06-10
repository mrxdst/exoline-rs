use std::path::Path;

pub fn split_once_and_trim_ascii(text: &str, delimiter: char) -> (&str, Option<&str>) {
    match text.split_once(delimiter) {
        None => (text.trim_ascii(), None),
        Some((a, b)) => (a.trim_ascii(), Some(b.trim_ascii())),
    }
}

pub async fn read_file_cp850(path: &Path) -> Result<String, std::io::Error> {
    let bytes = tokio::fs::read(path).await?;
    let content = oem_cp::decode_string_complete_table(bytes, &oem_cp::code_table::DECODING_TABLE_CP850);
    Ok(content)
}
