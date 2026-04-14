use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderCandidate {
    pub path: String,
    pub description: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFile {
    pub path: String,
    pub matched_lines: Vec<MatchedLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedLine {
    pub line_number: usize,
    pub content: String,
    pub is_filename_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub path: String,
    pub warning: Option<Warning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationCandidate {
    pub path: String,
    pub description: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveResult {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub old_name: String,
    pub new_name: String,
    pub warning: Option<Warning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitResult {
    pub source: String,
    pub shortcut: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub ts: String,
    pub file_path: String,
    pub message: String,
    pub rule_violated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntry {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEntry {
    pub ts: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderResult {
    pub path: String,
    pub description: String,
    pub is_new: bool,
}
