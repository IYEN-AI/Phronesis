use std::path::PathBuf;

use crate::error::{PhronesisError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub data_root: PathBuf,
    pub openai_api_key: Option<String>,
    pub embedding_model: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let data_root = std::env::var("PHRONESIS_DATA_ROOT")
            .map(PathBuf::from)
            .map_err(|_| PhronesisError::Config("PHRONESIS_DATA_ROOT not set".into()))?;

        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();

        let embedding_model = std::env::var("PHRONESIS_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".into());

        Ok(Self {
            data_root,
            openai_api_key,
            embedding_model,
        })
    }

    pub fn use_openai(&self) -> bool {
        self.openai_api_key
            .as_ref()
            .map_or(false, |k| !k.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn config_reads_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("PHRONESIS_EMBEDDING_MODEL") };
        unsafe { std::env::set_var("PHRONESIS_DATA_ROOT", "/tmp/phronesis-test") };
        let config = Config::from_env().unwrap();
        assert_eq!(config.data_root, PathBuf::from("/tmp/phronesis-test"));
        assert_eq!(config.embedding_model, "text-embedding-3-small");
    }

    #[test]
    fn config_from_env_missing_data_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("PHRONESIS_DATA_ROOT") };
        let result = Config::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("PHRONESIS_DATA_ROOT"),
            "Error should mention PHRONESIS_DATA_ROOT, got: {}",
            err
        );
    }

    #[test]
    fn config_custom_embedding_model() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("PHRONESIS_DATA_ROOT", "/tmp/test") };
        unsafe { std::env::set_var("PHRONESIS_EMBEDDING_MODEL", "text-embedding-3-large") };
        let config = Config::from_env().unwrap();
        assert_eq!(config.embedding_model, "text-embedding-3-large");
        unsafe { std::env::remove_var("PHRONESIS_EMBEDDING_MODEL") };
    }

    #[test]
    fn config_use_openai_checks_key() {
        let config = Config {
            data_root: PathBuf::from("/tmp"),
            openai_api_key: Some("sk-test".into()),
            embedding_model: "text-embedding-3-small".into(),
        };
        assert!(config.use_openai());

        let config_none = Config {
            data_root: PathBuf::from("/tmp"),
            openai_api_key: None,
            embedding_model: "text-embedding-3-small".into(),
        };
        assert!(!config_none.use_openai());

        let config_empty = Config {
            data_root: PathBuf::from("/tmp"),
            openai_api_key: Some("".into()),
            embedding_model: "text-embedding-3-small".into(),
        };
        assert!(!config_empty.use_openai());
    }

    #[test]
    fn config_struct_fields() {
        let config = Config {
            data_root: PathBuf::from("/custom/path"),
            openai_api_key: Some("key".into()),
            embedding_model: "custom-model".into(),
        };
        assert_eq!(config.data_root, PathBuf::from("/custom/path"));
        assert_eq!(config.openai_api_key, Some("key".into()));
        assert_eq!(config.embedding_model, "custom-model");
    }
}
