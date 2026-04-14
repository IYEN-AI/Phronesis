use phronesis::bootstrap;
use phronesis::config::Config;
use phronesis::search::embedding::{EmbeddingProvider, LocalEmbedding, OpenAIEmbedding};
use phronesis::search::vector_store::HnswStore;
use phronesis::server::PhronesisServer;
use rmcp::ServiceExt;

async fn build_index(
    data_root: &std::path::Path,
    store: &mut HnswStore,
    provider: &dyn EmbeddingProvider,
) -> anyhow::Result<()> {
    tracing::info!("Building initial embedding index from .meta.jsonl files");
    for entry in std::fs::read_dir(data_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && !path
                .file_name()
                .map_or(true, |n| n.to_string_lossy().starts_with('.'))
        {
            if let Ok(Some(description)) = phronesis::fs::meta::get_latest_description(&path) {
                match provider.embed(&description).await {
                    Ok(vec) => {
                        let rel_path = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        if !rel_path.is_empty() {
                            store.insert(rel_path, description, vec);
                        }
                    }
                    Err(e) => tracing::warn!("Failed to embed {}: {}", path.display(), e),
                }
            }
        }
    }
    if let Err(e) = store.save() {
        tracing::warn!("Failed to save initial index: {}", e);
    }
    Ok(())
}

async fn run_server(
    config: Config,
    store: HnswStore,
    provider: impl EmbeddingProvider + 'static,
) -> anyhow::Result<()> {
    let server = PhronesisServer::new(config, store, provider);
    tracing::info!("Starting Phronesis MCP server on stdio");
    let transport = rmcp::transport::io::stdio();
    let server_handle = server.serve(transport).await?;
    server_handle.waiting().await?;
    Ok(())
}

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

    if config.use_openai() {
        let key = config
            .openai_api_key
            .clone()
            .expect("openai_api_key must be Some when use_openai() is true");
        let provider = OpenAIEmbedding::new(key, config.embedding_model.clone());
        tracing::info!("Using OpenAI embedding ({})", config.embedding_model);
        if store.is_empty() {
            build_index(&config.data_root, &mut store, &provider).await?;
        }
        run_server(config, store, provider).await
    } else {
        let provider = LocalEmbedding::new()?;
        tracing::info!("Using local embedding (MultilingualE5Small, no API key needed)");
        if store.is_empty() {
            build_index(&config.data_root, &mut store, &provider).await?;
        }
        run_server(config, store, provider).await
    }
}
