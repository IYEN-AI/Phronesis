use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderCandidate {
    pub path: String,
    pub description: String,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFile {
    pub path: String,
    pub matched_lines: Vec<MatchedLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedLine {
    pub line_number: usize,
    pub line: String,
    #[serde(default)]
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
    pub moved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
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
    #[serde(default)]
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub path: String,
    pub ts: String,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub rule_violated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntry {
    pub description: String,
    pub created: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEntry {
    pub ts: String,
    pub situation: String,
    pub reasoning: String,
    pub action: String,
    pub outcome: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderResult {
    pub path: String,
    pub description: String,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionKindFilter {
    Wisdom,
    Decision,
    TemporalRollup,
    Report,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Wisdom,
    Decision,
    TemporalRollup,
    Report,
}

impl ArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactKind::Wisdom => "wisdom",
            ArtifactKind::Decision => "decision",
            ArtifactKind::TemporalRollup => "temporal_rollup",
            ArtifactKind::Report => "report",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    Active,
    Superseded,
    Stale,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionJobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Retrying,
    Cancelled,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    Supports,
    Contradicts,
    Context,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Semantic,
    Temporal,
    Docs,
    Decision,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    Public,
    Operator,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceWatermark {
    pub content_hash: String,
    pub latest_mtime: String,
    pub source_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionScope {
    pub agent: String,
    #[serde(default)]
    pub source_paths: Vec<String>,
    #[serde(default)]
    pub granularity: Option<String>,
    #[serde(default)]
    pub period_start: Option<String>,
    #[serde(default)]
    pub period_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionEnqueueRequestPayload {
    pub kind: ProjectionKindFilter,
    pub scope: ProjectionScope,
    pub idempotency_key: String,
    pub source_watermark: SourceWatermark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionEnqueueResult {
    pub job_id: String,
    pub status: ProjectionJobStatus,
    pub accepted: bool,
    pub idempotency_key: String,
    #[serde(default)]
    pub existing_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    pub claim_id: String,
    pub text: String,
    pub scope: String,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSpan {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub source_kind: SourceKind,
    pub agent: String,
    pub path: String,
    #[serde(default)]
    pub record_id: Option<String>,
    #[serde(default)]
    pub span: Option<EvidenceSpan>,
    pub excerpt: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceEdge {
    pub claim_id: String,
    pub evidence_id: String,
    pub relation: EvidenceRelation,
    #[serde(default)]
    pub weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EvidenceGraph {
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default)]
    pub edges: Vec<EvidenceEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionArtifact {
    pub schema_version: u32,
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub agent: String,
    pub created_at: String,
    pub source_watermark: SourceWatermark,
    pub idempotency_key: String,
    pub content_hash: String,
    pub status: ArtifactStatus,
    pub freshness: Freshness,
    pub evidence_graph: EvidenceGraph,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetryEvent {
    pub at: String,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionJob {
    pub job_id: String,
    pub agent: String,
    pub kind: ProjectionKindFilter,
    pub status: ProjectionJobStatus,
    pub requested_at: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    pub retry_count: usize,
    #[serde(default)]
    pub retry_history: Vec<RetryEvent>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub watermark_before: SourceWatermark,
    #[serde(default)]
    pub watermark_after: Option<SourceWatermark>,
    pub stale: bool,
    #[serde(default)]
    pub stale_reason: Option<String>,
    pub idempotency_key: String,
    #[serde(default)]
    pub produced_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionGetJobRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionListJobsRequest {
    pub agent: String,
    #[serde(default)]
    pub kind: Option<ProjectionKindFilter>,
    #[serde(default)]
    pub status: Option<ProjectionJobStatus>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionGetArtifactRequest {
    pub artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionListArtifactsRequest {
    pub agent: String,
    #[serde(default)]
    pub kind: Option<ArtifactKind>,
    #[serde(default)]
    pub status: Option<ArtifactStatus>,
    #[serde(default)]
    pub granularity: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionSearchArtifactsRequest {
    pub agent: String,
    pub q: String,
    #[serde(default)]
    pub kind: Option<ArtifactKind>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionRefreshStaleRequest {
    pub agent: String,
    pub kind: ProjectionKindFilter,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionArtifactListResponse {
    pub artifacts: Vec<ProjectionArtifact>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionJobListResponse {
    pub jobs: Vec<ProjectionJob>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionSearchResult {
    pub artifact: ProjectionArtifact,
    pub score: f32,
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionSearchResponse {
    pub results: Vec<ProjectionSearchResult>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionRefreshStaleResponse {
    pub enqueued_job_ids: Vec<String>,
    pub skipped_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub details: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectionEnvelope<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub error: Option<ProjectionErrorBody>,
}
