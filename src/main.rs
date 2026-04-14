use phronesis::bootstrap;
use phronesis::config::Config;
use phronesis::search::embedding::{EmbeddingProvider, OpenAIEmbedding};
use phronesis::search::vector_store::HnswStore;
use phronesis::server::PhronesisServer;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;

    // Bootstrap 6-Pillar filesystem
    bootstrap::bootstrap(&config.data_root)?;
    tracing::info!("Phronesis bootstrapped at {:?}", config.data_root);

    // Initialize or load vector index
    let index_dir = config.data_root.join(".index");
    let mut store = HnswStore::load(&index_dir).unwrap_or_else(|_| {
        tracing::warn!("Failed to load index, creating fresh");
        HnswStore::new(&index_dir)
    });

    // If store is empty, build initial index from .meta.jsonl files
    let provider = OpenAIEmbedding::new(
        config.openai_api_key.clone(),
        config.embedding_model.clone(),
    );

    if store.is_empty() {
        tracing::info!("Building initial embedding index from .meta.jsonl files");
        for entry in std::fs::read_dir(&config.data_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && !path
                    .file_name()
                    .map_or(true, |n| n.to_string_lossy().starts_with('.'))
            {
                if let Ok(Some(description)) =
                    phronesis::fs::meta::get_latest_description(&path)
                {
                    match provider.embed(&description).await {
                        Ok(vec) => {
                            let rel_path =
                                path.file_name().unwrap().to_string_lossy().to_string();
                            store.insert(rel_path, description, vec);
                        }
                        Err(e) => tracing::warn!("Failed to embed {}: {}", path.display(), e),
                    }
                }
            }
        }
        if let Err(e) = store.save() {
            tracing::warn!("Failed to save initial index: {}", e);
        }
    }

    // Create server and serve via stdio
    let server = PhronesisServer::new(config, store, provider);
    tracing::info!("Starting Phronesis MCP server on stdio");

    let transport = rmcp::transport::io::stdio();
    let server_handle = server.serve(transport).await?;
    server_handle.waiting().await?;

    Ok(())
}
