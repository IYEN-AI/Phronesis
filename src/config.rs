use std::path::PathBuf;

use crate::error::{PhronesisError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub data_root: PathBuf,
    pub openai_api_key: String,
    pub embedding_model: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let data_root = std::env::var("PHRONESIS_DATA_ROOT")
            .map(PathBuf::from)
            .map_err(|_| PhronesisError::Config("PHRONESIS_DATA_ROOT not set".into()))?;

        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| PhronesisError::Config("OPENAI_API_KEY not set".into()))?;

        let embedding_model = std::env::var("PHRONESIS_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".into());

        Ok(Self {
            data_root,
            openai_api_key,
            embedding_model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_reads_from_env() {
        std::env::set_var("PHRONESIS_DATA_ROOT", "/tmp/phronesis-test");
        std::env::set_var("OPENAI_API_KEY", "test-key");
        let config = Config::from_env().unwrap();
        assert_eq!(config.data_root, PathBuf::from("/tmp/phronesis-test"));
        assert_eq!(config.openai_api_key, "test-key");
        assert_eq!(config.embedding_model, "text-embedding-3-small");
    }
}
