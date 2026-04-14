use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, tool_router, ServerHandler};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::evolution::{habit, move_action, rename_action};
use crate::fs::{action, meta, naming, warnings};
use crate::search::embedding::EmbeddingProvider;
use crate::search::{grep, suggest, vector_store::HnswStore};

#[allow(dead_code)]
pub struct PhronesisServer {
    tool_router: ToolRouter<Self>,
    config: Config,
    store: Arc<RwLock<HnswStore>>,
    provider: Arc<dyn EmbeddingProvider>,
}

impl PhronesisServer {
    pub fn new(config: Config, store: HnswStore, provider: impl EmbeddingProvider + 'static) -> Self {
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

// -- Tool implementations --

#[tool_router]
impl PhronesisServer {
    #[tool(description = "Search for relevant folders using semantic embedding. Returns folders whose descriptions are most similar to the query. Use this for situation awareness: 'What context am I in?'")]
    async fn embed_search(
        &self,
        Parameters(req): Parameters<EmbedSearchRequest>,
    ) -> String {
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

    #[tool(description = "Search for action files by regex pattern in filenames and content. Use this for action selection: 'What should I do?'")]
    fn grep_search(
        &self,
        Parameters(req): Parameters<GrepSearchRequest>,
    ) -> String {
        let folder = self.data_root().join(&req.folder);
        match grep::grep_search(&folder, &req.pattern, req.max_results) {
            Ok(results) => serde_json::to_string_pretty(&results).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Read the full trajectory (all JSONL entries) of an action file.")]
    fn read_action(
        &self,
        Parameters(req): Parameters<ReadActionRequest>,
    ) -> String {
        let path = self.data_root().join(&req.path);
        match action::read_action(&path) {
            Ok(entries) => serde_json::to_string_pretty(&entries).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Append a new entry to an action file (append-only, no deletion). Returns a warning if the filename violates naming conventions.")]
    fn write_action(
        &self,
        Parameters(req): Parameters<WriteActionRequest>,
    ) -> String {
        let path = self.data_root().join(&req.path);

        // Validate naming convention
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
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

    #[tool(description = "Suggest the best folder location for a new action based on a description. Returns ranked candidates.")]
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

    #[tool(description = "Create a new folder with a description. If the folder already exists, updates its description (idempotent).")]
    async fn create_folder(
        &self,
        Parameters(req): Parameters<CreateFolderRequest>,
    ) -> String {
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

    #[tool(description = "Move a file or folder to a new location. Automatically updates the embedding index for folder moves.")]
    async fn move_action(
        &self,
        Parameters(req): Parameters<MoveActionRequest>,
    ) -> String {
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

    #[tool(description = "Rename an action file. Validates naming convention and logs a warning if violated.")]
    fn rename_action(
        &self,
        Parameters(req): Parameters<RenameActionRequest>,
    ) -> String {
        match rename_action::rename_action(self.data_root(), &req.path, &req.new_name) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Create a symlink shortcut (habit) to a frequently used action file. Enables quick access without search.")]
    fn create_habit(
        &self,
        Parameters(req): Parameters<CreateHabitRequest>,
    ) -> String {
        match habit::create_habit(self.data_root(), &req.source, &req.shortcut) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get naming convention violation warnings. Useful for reflection and self-improvement.")]
    fn get_warnings(
        &self,
        Parameters(req): Parameters<GetWarningsRequest>,
    ) -> String {
        match warnings::get_warnings(self.data_root(), req.since.as_deref()) {
            Ok(warnings) => serde_json::to_string_pretty(&warnings).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }
}

impl ServerHandler for PhronesisServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Phronesis: An agentic memory system based on Aristotle's unity of knowledge and action. Knowledge is stored as executable action files in a 6-Pillar filesystem. Use embed_search for situation awareness, grep_search for action selection, and write_action to record new experiences.")
    }
}
