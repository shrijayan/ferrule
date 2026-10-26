//! Models several resources share.

use serde::{Deserialize, Serialize};

/// A note.
///
/// - It has blocks
///   and a parent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<std::collections::BTreeMap<String, String>>,
    /// Plain text or one block.
    pub content: NoteContent,
    pub created_at: String,
    pub id: String,
    /// The note this one answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<crate::shared::Note>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub title: String,
}

/// Plain text or one block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum NoteContent {
    String(String),
    NoteBlock(crate::shared::NoteBlock),
}

/// A block of rich text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteBlock {
    pub text: String,
    pub r#type: NoteBlockType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteBlockType {
    #[serde(rename = "paragraph")]
    Paragraph,
    #[serde(rename = "heading")]
    Heading,
    #[serde(rename = "code-block")]
    CodeBlock,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}
