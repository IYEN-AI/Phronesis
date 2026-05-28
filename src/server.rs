use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, tool_router, ServerHandler};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::evolution::{habit, move_action, rename_action};
use crate::fs::{action, meta, naming, projection, warnings};
use crate::search::embedding::EmbeddingProvider;
use crate::search::{grep, suggest, vector_store::HnswStore};
use crate::types::{
    ArtifactKind, ArtifactStatus, ProjectionEnqueueRequestPayload, ProjectionJobStatus,
    ProjectionKindFilter, ProjectionRefreshStaleRequest, ProjectionScope,
    ProjectionSearchArtifactsRequest, SourceWatermark,
};

#[allow(dead_code)]
pub struct PhronesisServer {
    tool_router: ToolRouter<Self>,
    config: Config,
    store: Arc<RwLock<HnswStore>>,
    provider: Arc<dyn EmbeddingProvider>,
}

impl PhronesisServer {
    pub fn new(
        config: Config,
        store: HnswStore,
        provider: impl EmbeddingProvider + 'static,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            config,
            store: Arc::new(RwLock::new(store)),
            provider: Arc::new(provider),
        }
    }

    fn data_root(&self) -> &PathBuf {
        &self.config.data_root
    }
}

// -- Request types --

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmbedSearchRequest {
    #[schemars(description = "Natural language query describing the situation or context")]
    pub query: String,
    #[schemars(description = "Number of top results to return (default: 5)")]
    pub top_k: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GrepSearchRequest {
    #[schemars(description = "Folder path to search within (relative to data root)")]
    pub folder: String,
    #[schemars(description = "Regex pattern to match against filenames and content")]
    pub pattern: String,
    #[schemars(description = "Maximum number of results to return (default: 50)")]
    pub max_results: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadActionRequest {
    #[schemars(description = "Path to the action file (relative to data root)")]
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteActionRequest {
    #[schemars(description = "Path to the action file (relative to data root)")]
    pub path: String,
    #[schemars(description = "JSON content to append as a new JSONL entry")]
    pub content: serde_json::Value,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SuggestLocationRequest {
    #[schemars(description = "Description of the action to find a location for")]
    pub description: String,
    #[schemars(description = "Number of top candidates to return (default: 5)")]
    pub top_k: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateFolderRequest {
    #[schemars(description = "Path for the new folder (relative to data root)")]
    pub path: String,
    #[schemars(description = "Short description of the folder's purpose (used for embedding)")]
    pub description: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MoveActionRequest {
    #[schemars(description = "Current path (relative to data root)")]
    pub old_path: String,
    #[schemars(description = "New path (relative to data root)")]
    pub new_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RenameActionRequest {
    #[schemars(description = "Current file path (relative to data root)")]
    pub path: String,
    #[schemars(description = "New filename (e.g., verb_object_method.jsonl)")]
    pub new_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateHabitRequest {
    #[schemars(description = "Source path to link to (relative to data root)")]
    pub source: String,
    #[schemars(description = "Shortcut path to create (relative to data root)")]
    pub shortcut: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetWarningsRequest {
    #[schemars(description = "Only return warnings after this ISO 8601 timestamp")]
    pub since: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionToolScope {
    pub agent: String,
    pub source_paths: Vec<String>,
    pub granularity: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionToolWatermark {
    pub content_hash: String,
    pub latest_mtime: String,
    pub source_count: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionEnqueueToolRequest {
    pub kind: String,
    pub scope: ProjectionToolScope,
    pub idempotency_key: String,
    pub source_watermark: ProjectionToolWatermark,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionGetJobToolRequest {
    pub job_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionListJobsToolRequest {
    pub agent: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionGetArtifactToolRequest {
    pub artifact_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionListArtifactsToolRequest {
    pub agent: String,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub granularity: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionSearchArtifactsToolRequest {
    pub agent: String,
    pub q: String,
    pub kind: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectionRefreshStaleToolRequest {
    pub agent: String,
    pub kind: String,
    pub limit: Option<usize>,
}

// -- Tool implementations --

#[tool_router]
impl PhronesisServer {
    #[tool(
        description = "Search for relevant folders using semantic embedding. Returns folders whose descriptions are most similar to the query. Use this for situation awareness: 'What context am I in?'"
    )]
    async fn embed_search(&self, Parameters(req): Parameters<EmbedSearchRequest>) -> String {
        let top_k = req.top_k.unwrap_or(5);
        let store = self.store.read().await;
        match self.provider.embed(&req.query).await {
            Ok(vec) => {
                let results = store.search(&vec, top_k);
                let candidates: Vec<_> = results
                    .into_iter()
                    .map(|(entry, score)| {
                        serde_json::json!({
                            "path": entry.path,
                            "description": entry.description,
                            "similarity": 1.0 - score,
                        })
                    })
                    .collect();
                serde_json::to_string_pretty(&candidates).unwrap_or_default()
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Search for action files by regex pattern in filenames and content. Use this for action selection: 'What should I do?'"
    )]
    fn grep_search(&self, Parameters(req): Parameters<GrepSearchRequest>) -> String {
        let folder = self.data_root().join(&req.folder);
        match grep::grep_search(&folder, &req.pattern, req.max_results) {
            Ok(results) => serde_json::to_string_pretty(&results).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Read the full trajectory (all JSONL entries) of an action file.")]
    fn read_action(&self, Parameters(req): Parameters<ReadActionRequest>) -> String {
        let path = self.data_root().join(&req.path);
        match action::read_action(&path) {
            Ok(entries) => serde_json::to_string_pretty(&entries).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Append a new entry to an action file (append-only, no deletion). Returns a warning if the filename violates naming conventions."
    )]
    fn write_action(&self, Parameters(req): Parameters<WriteActionRequest>) -> String {
        let path = self.data_root().join(&req.path);

        // Validate naming convention
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let warning = naming::validate_name(filename, &req.path);

        // Log warning if naming violated
        if let Some(ref w) = warning {
            let _ = warnings::log_warning(self.data_root(), w);
        }

        match action::append_action(&path, &req.content) {
            Ok(()) => {
                let result = crate::types::WriteResult {
                    path: req.path,
                    warning,
                };
                serde_json::to_string_pretty(&result).unwrap_or_default()
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Suggest the best folder location for a new action based on a description. Returns ranked candidates."
    )]
    async fn suggest_location(
        &self,
        Parameters(req): Parameters<SuggestLocationRequest>,
    ) -> String {
        let top_k = req.top_k.unwrap_or(5);
        let store = self.store.read().await;
        match suggest::suggest_location(&req.description, self.provider.as_ref(), &store, top_k)
            .await
        {
            Ok(candidates) => serde_json::to_string_pretty(&candidates).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Create a new folder with a description. If the folder already exists, updates its description (idempotent)."
    )]
    async fn create_folder(&self, Parameters(req): Parameters<CreateFolderRequest>) -> String {
        let folder_path = self.data_root().join(&req.path);
        let is_new = !folder_path.exists();

        if let Err(e) = std::fs::create_dir_all(&folder_path) {
            return format!("Error: {}", e);
        }

        if let Err(e) = meta::append_meta(&folder_path, &req.description) {
            return format!("Error: {}", e);
        }

        // Update embedding index
        match self.provider.embed(&req.description).await {
            Ok(vec) => {
                let mut store = self.store.write().await;
                store.insert(req.path.clone(), req.description.clone(), vec);
            }
            Err(e) => return format!("Error embedding: {}", e),
        }

        let result = crate::types::FolderResult {
            path: req.path,
            description: req.description,
            is_new,
        };
        serde_json::to_string_pretty(&result).unwrap_or_default()
    }

    #[tool(
        description = "Move a file or folder to a new location. Automatically updates the embedding index for folder moves."
    )]
    async fn move_action(&self, Parameters(req): Parameters<MoveActionRequest>) -> String {
        let mut store = self.store.write().await;
        match move_action::move_action(
            self.data_root(),
            &req.old_path,
            &req.new_path,
            &mut store,
            self.provider.as_ref(),
        )
        .await
        {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Rename an action file. Validates naming convention and logs a warning if violated."
    )]
    fn rename_action(&self, Parameters(req): Parameters<RenameActionRequest>) -> String {
        match rename_action::rename_action(self.data_root(), &req.path, &req.new_name) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Create a symlink shortcut (habit) to a frequently used action file. Enables quick access without search."
    )]
    fn create_habit(&self, Parameters(req): Parameters<CreateHabitRequest>) -> String {
        match habit::create_habit(self.data_root(), &req.source, &req.shortcut) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(
        description = "Enqueue and persist phronesis projection artifacts for wisdom, decisions, temporal rollups, reports, or all of them for a source scope."
    )]
    fn projection_enqueue(
        &self,
        Parameters(req): Parameters<ProjectionEnqueueToolRequest>,
    ) -> String {
        match map_enqueue_request(req)
            .and_then(|payload| projection::enqueue(self.data_root(), payload))
        {
            Ok(result) => projection::ok_envelope(result),
            Err(error) => projection_error_envelope(error),
        }
    }

    #[tool(description = "Read a persisted phronesis projection job by id.")]
    fn projection_get_job(
        &self,
        Parameters(req): Parameters<ProjectionGetJobToolRequest>,
    ) -> String {
        match projection::get_job(self.data_root(), &req.job_id) {
            Ok(job) => projection::ok_envelope(job),
            Err(error) => projection_error_envelope(error),
        }
    }

    #[tool(
        description = "List persisted phronesis projection jobs for an agent, optionally filtered by kind and status."
    )]
    fn projection_list_jobs(
        &self,
        Parameters(req): Parameters<ProjectionListJobsToolRequest>,
    ) -> String {
        let result = parse_projection_kind_filter_opt(req.kind.as_deref()).and_then(|kind| {
            parse_projection_job_status_opt(req.status.as_deref()).and_then(|status| {
                projection::list_jobs(
                    self.data_root(),
                    &req.agent,
                    kind,
                    status,
                    req.limit.unwrap_or(50).max(1),
                    req.offset.unwrap_or(0),
                )
            })
        });
        match result {
            Ok(response) => projection::ok_envelope(response),
            Err(error) => projection_error_envelope(error),
        }
    }

    #[tool(description = "Read a persisted phronesis projection artifact by id.")]
    fn projection_get_artifact(
        &self,
        Parameters(req): Parameters<ProjectionGetArtifactToolRequest>,
    ) -> String {
        match projection::get_artifact(self.data_root(), &req.artifact_id) {
            Ok(artifact) => projection::ok_envelope(artifact),
            Err(error) => projection_error_envelope(error),
        }
    }

    #[tool(
        description = "List persisted phronesis projection artifacts for an agent, optionally filtered by kind, status, and granularity."
    )]
    fn projection_list_artifacts(
        &self,
        Parameters(req): Parameters<ProjectionListArtifactsToolRequest>,
    ) -> String {
        let result = parse_artifact_kind_opt(req.kind.as_deref()).and_then(|kind| {
            parse_artifact_status_opt(req.status.as_deref()).and_then(|status| {
                projection::list_artifacts(
                    self.data_root(),
                    &req.agent,
                    kind,
                    status,
                    req.granularity.as_deref(),
                    req.limit.unwrap_or(50).max(1),
                    req.offset.unwrap_or(0),
                )
            })
        });
        match result {
            Ok(response) => projection::ok_envelope(response),
            Err(error) => projection_error_envelope(error),
        }
    }

    #[tool(description = "Search persisted phronesis projection artifacts for an agent.")]
    fn projection_search_artifacts(
        &self,
        Parameters(req): Parameters<ProjectionSearchArtifactsToolRequest>,
    ) -> String {
        let result = parse_artifact_kind_opt(req.kind.as_deref()).and_then(|kind| {
            projection::search_artifacts(
                self.data_root(),
                ProjectionSearchArtifactsRequest {
                    agent: req.agent,
                    q: req.q,
                    kind,
                    limit: req.limit,
                },
            )
        });
        match result {
            Ok(response) => projection::ok_envelope(response),
            Err(error) => projection_error_envelope(error),
        }
    }

    #[tool(description = "Refresh stale projection artifacts for an agent by re-enqueueing them.")]
    fn projection_refresh_stale(
        &self,
        Parameters(req): Parameters<ProjectionRefreshStaleToolRequest>,
    ) -> String {
        let result = parse_projection_kind_filter(req.kind.as_str()).and_then(|kind| {
            projection::refresh_stale(
                self.data_root(),
                ProjectionRefreshStaleRequest {
                    agent: req.agent,
                    kind,
                    limit: req.limit,
                },
            )
        });
        match result {
            Ok(response) => projection::ok_envelope(response),
            Err(error) => projection_error_envelope(error),
        }
    }
    #[tool(
        description = "Get naming convention violation warnings. Useful for reflection and self-improvement."
    )]
    fn get_warnings(&self, Parameters(req): Parameters<GetWarningsRequest>) -> String {
        match warnings::get_warnings(self.data_root(), req.since.as_deref()) {
            Ok(warnings) => serde_json::to_string_pretty(&warnings).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }
}

fn map_enqueue_request(
    req: ProjectionEnqueueToolRequest,
) -> crate::error::Result<ProjectionEnqueueRequestPayload> {
    Ok(ProjectionEnqueueRequestPayload {
        kind: parse_projection_kind_filter(req.kind.as_str())?,
        scope: ProjectionScope {
            agent: req.scope.agent,
            source_paths: req.scope.source_paths,
            granularity: req.scope.granularity,
            period_start: req.scope.period_start,
            period_end: req.scope.period_end,
        },
        idempotency_key: req.idempotency_key,
        source_watermark: SourceWatermark {
            content_hash: req.source_watermark.content_hash,
            latest_mtime: req.source_watermark.latest_mtime,
            source_count: req.source_watermark.source_count,
        },
    })
}

fn parse_projection_kind_filter(value: &str) -> crate::error::Result<ProjectionKindFilter> {
    match value {
        "wisdom" => Ok(ProjectionKindFilter::Wisdom),
        "decision" => Ok(ProjectionKindFilter::Decision),
        "temporal_rollup" => Ok(ProjectionKindFilter::TemporalRollup),
        "report" => Ok(ProjectionKindFilter::Report),
        "all" => Ok(ProjectionKindFilter::All),
        other => Err(crate::error::PhronesisError::Validation(format!(
            "invalid projection kind: {other}"
        ))),
    }
}

fn parse_projection_kind_filter_opt(
    value: Option<&str>,
) -> crate::error::Result<Option<ProjectionKindFilter>> {
    value.map(parse_projection_kind_filter).transpose()
}

fn parse_artifact_kind_opt(value: Option<&str>) -> crate::error::Result<Option<ArtifactKind>> {
    value
        .map(|value| match value {
            "wisdom" => Ok(ArtifactKind::Wisdom),
            "decision" => Ok(ArtifactKind::Decision),
            "temporal_rollup" => Ok(ArtifactKind::TemporalRollup),
            "report" => Ok(ArtifactKind::Report),
            other => Err(crate::error::PhronesisError::Validation(format!(
                "invalid artifact kind: {other}"
            ))),
        })
        .transpose()
}

fn parse_artifact_status_opt(value: Option<&str>) -> crate::error::Result<Option<ArtifactStatus>> {
    value
        .map(|value| match value {
            "active" => Ok(ArtifactStatus::Active),
            "superseded" => Ok(ArtifactStatus::Superseded),
            "stale" => Ok(ArtifactStatus::Stale),
            "rejected" => Ok(ArtifactStatus::Rejected),
            other => Err(crate::error::PhronesisError::Validation(format!(
                "invalid artifact status: {other}"
            ))),
        })
        .transpose()
}

fn parse_projection_job_status_opt(
    value: Option<&str>,
) -> crate::error::Result<Option<ProjectionJobStatus>> {
    value
        .map(|value| match value {
            "queued" => Ok(ProjectionJobStatus::Queued),
            "running" => Ok(ProjectionJobStatus::Running),
            "succeeded" => Ok(ProjectionJobStatus::Succeeded),
            "failed" => Ok(ProjectionJobStatus::Failed),
            "retrying" => Ok(ProjectionJobStatus::Retrying),
            "cancelled" => Ok(ProjectionJobStatus::Cancelled),
            "stale" => Ok(ProjectionJobStatus::Stale),
            other => Err(crate::error::PhronesisError::Validation(format!(
                "invalid projection job status: {other}"
            ))),
        })
        .transpose()
}

fn projection_error_envelope(error: crate::error::PhronesisError) -> String {
    let retryable = matches!(
        error,
        crate::error::PhronesisError::Embedding(_) | crate::error::PhronesisError::Io(_)
    );
    let code = match error {
        crate::error::PhronesisError::Validation(_) => "invalid_request",
        crate::error::PhronesisError::NotFound(_) => "not_found",
        crate::error::PhronesisError::Embedding(_) => "unavailable",
        crate::error::PhronesisError::Io(_) => "internal",
        crate::error::PhronesisError::Json(_) => "internal",
        crate::error::PhronesisError::Config(_) => "internal",
    };
    projection::err_envelope(code, error.to_string(), retryable, serde_json::json!({}))
}

#[rmcp::tool_handler(router = Self::tool_router())]
impl ServerHandler for PhronesisServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Phronesis: An agentic memory system based on Aristotle's unity of knowledge and action. Knowledge is stored as executable action files in a 6-Pillar filesystem. Use embed_search for situation awareness, grep_search for action selection, and write_action to record new experiences.")
    }
}
