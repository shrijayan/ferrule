//! `multipart/form-data` bodies: files and fields, encoded by the runtime so that a request can
//! be sent again when it is retried.

use serde_json::Value;

use super::query;

/// A file to upload: its name, its content and, optionally, its media type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUpload {
    /// The file name the server sees.
    pub filename: String,
    /// The media type; `application/octet-stream` when not set.
    pub content_type: Option<String>,
    pub bytes: bytes::Bytes,
}

impl FileUpload {
    /// A file named `filename` with content `bytes`.
    pub fn new(filename: impl Into<String>, bytes: impl Into<bytes::Bytes>) -> Self {
        Self {
            filename: filename.into(),
            content_type: None,
            bytes: bytes.into(),
        }
    }

    /// The file with media type `content_type`.
    #[must_use]
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// The file at `path`, named after its last component.
    ///
    /// # Errors
    ///
    /// The I/O error when the file cannot be read.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        let name = path
            .file_name()
            .map_or_else(|| "file".to_owned(), |n| n.to_string_lossy().into_owned());
        Ok(Self::new(name, bytes))
    }
}

/// One part of a multipart body.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct Part {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub bytes: bytes::Bytes,
}

/// The parts of field `name` with JSON value `value`: one per array item, objects as JSON, and
/// none for `null`.
pub(crate) fn field_parts(name: &str, value: &Value, out: &mut Vec<Part>) {
    match value {
        Value::Null => {}
        Value::Array(items) => items.iter().for_each(|v| field_parts(name, v, out)),
        Value::Object(_) => out.push(text_part(name, value.to_string())),
        other => out.extend(query::scalar(other).map(|text| text_part(name, text))),
    }
}

fn text_part(name: &str, text: String) -> Part {
    Part {
        name: name.to_owned(),
        filename: None,
        content_type: None,
        bytes: text.into(),
    }
}

/// The part of file `file` in field `name`.
pub(crate) fn file_part(name: &str, file: &FileUpload) -> Part {
    Part {
        name: name.to_owned(),
        filename: Some(file.filename.clone()),
        content_type: Some(
            file.content_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        ),
        bytes: file.bytes.clone(),
    }
}

/// A header parameter value: no quotes, and no line breaks that would end the header.
fn quoted(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '"' => "%22".to_owned(),
            '\r' => "%0D".to_owned(),
            '\n' => "%0A".to_owned(),
            c => c.to_string(),
        })
        .collect()
}

/// The body of `parts` with `boundary`, and its content type.
pub(crate) fn encode(parts: &[Part], boundary: &str) -> (String, bytes::Bytes) {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        let mut disposition = format!(
            "Content-Disposition: form-data; name=\"{}\"",
            quoted(&part.name)
        );
        if let Some(filename) = &part.filename {
            disposition.push_str(&format!("; filename=\"{}\"", quoted(filename)));
        }
        out.extend_from_slice(disposition.as_bytes());
        out.extend_from_slice(b"\r\n");
        if let Some(content_type) = &part.content_type {
            let content_type: String = content_type.chars().filter(|c| !c.is_control()).collect();
            out.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&part.bytes);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (
        format!("multipart/form-data; boundary={boundary}"),
        out.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encodes_fields_and_files() {
        let mut parts = Vec::new();
        field_parts("purpose", &json!("notes"), &mut parts);
        field_parts("tags", &json!(["a", 1]), &mut parts);
        field_parts("skip", &Value::Null, &mut parts);
        field_parts("meta", &json!({"k": true}), &mut parts);
        let file = FileUpload::new("a\"b\r\n.txt", b"hi".to_vec()).content_type("text/plain");
        parts.push(file_part("file", &file));
        let (content_type, body) = encode(&parts, "XYZ");
        assert_eq!(content_type, "multipart/form-data; boundary=XYZ");
        let expected = "--XYZ\r\nContent-Disposition: form-data; name=\"purpose\"\r\n\r\nnotes\r\n\
             --XYZ\r\nContent-Disposition: form-data; name=\"tags\"\r\n\r\na\r\n\
             --XYZ\r\nContent-Disposition: form-data; name=\"tags\"\r\n\r\n1\r\n\
             --XYZ\r\nContent-Disposition: form-data; name=\"meta\"\r\n\r\n{\"k\":true}\r\n\
             --XYZ\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a%22b%0D%0A.txt\"\r\n\
             Content-Type: text/plain\r\n\r\nhi\r\n--XYZ--\r\n";
        assert_eq!(String::from_utf8_lossy(&body), expected);
    }

    #[test]
    fn reads_a_file_from_disk() {
        let path = std::env::temp_dir().join(format!("rt-multipart-{}.txt", std::process::id()));
        std::fs::write(&path, b"x").expect("writes");
        let file = FileUpload::from_path(&path).expect("reads");
        std::fs::remove_file(&path).expect("removes");
        assert_eq!((file.bytes.as_ref(), file.content_type), (&b"x"[..], None));
        assert!(file.filename.starts_with("rt-multipart-"));
    }
}
