// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SecureBackend {
    GitHub(GitHubConfig),
    InMemoryStorage,
    Vault(VaultConfig),
    OnDiskStorage(OnDiskStorageConfig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GitHubConfig {
    /// The owner or account that hosts a repository
    pub owner: String,
    /// The repository where storage will mount
    pub repository: String,
    /// The authorization token for accessing the repository
    pub token: Token,
    /// A namespace is an optional portion of the path to a key stored within OnDiskStorage. For
    /// example, a key, S, without a namespace would be available in S, with a namespace, N, it
    /// would be in N/S.
    pub namespace: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VaultConfig {
    /// Optional SSL Certificate for the vault host, this is expected to be a full path.
    pub ca_certificate: Option<PathBuf>,
    /// A namespace is an optional portion of the path to a key stored within Vault. For example,
    /// a secret, S, without a namespace would be available in secret/data/S, with a namespace, N, it
    /// would be in secret/data/N/S.
    pub namespace: Option<String>,
    /// Vault's URL, note: only HTTP is currently supported.
    pub server: String,
    /// The authorization token for accessing secrets
    pub token: Token,
}

impl VaultConfig {
    pub fn ca_certificate(&self) -> Result<String> {
        let path = self
            .ca_certificate
            .as_ref()
            .ok_or_else(|| anyhow!("No Certificate path"))?;
        read_file(path)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OnDiskStorageConfig {
    // Required path for on disk storage
    pub path: PathBuf,
    /// A namespace is an optional portion of the path to a key stored within OnDiskStorage. For
    /// example, a key, S, without a namespace would be available in S, with a namespace, N, it
    /// would be in N/S.
    pub namespace: Option<String>,
    #[serde(skip)]
    data_dir: PathBuf,
}

/// Tokens can either be directly within this config or stored somewhere on disk.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Token {
    FromConfig(String),
    /// This is an absolute path and not relative to data_dir
    FromDisk(PathBuf),
}

impl Token {
    pub fn read_token(&self) -> Result<String> {
        match self {
            Token::FromDisk(path) => read_file(path),
            Token::FromConfig(token) => Ok(token.clone()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TokenFromConfig {
    token: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TokenFromDisk {
    path: PathBuf,
}

impl Default for OnDiskStorageConfig {
    fn default() -> Self {
        Self {
            namespace: None,
            path: PathBuf::from("secure_storage.toml"),
            data_dir: PathBuf::from("/opt/libra/data/common"),
        }
    }
}

impl OnDiskStorageConfig {
    pub fn path(&self) -> PathBuf {
        if self.path.is_relative() {
            self.data_dir.join(&self.path)
        } else {
            self.path.clone()
        }
    }

    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.data_dir = data_dir;
    }
}

fn read_file(path: &PathBuf) -> Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Config {
        vault: VaultConfig,
    }

    #[test]
    fn test_token_config_parsing() {
        let from_config = Config {
            vault: VaultConfig {
                namespace: None,
                server: "127.0.0.1:8200".to_string(),
                ca_certificate: None,
                token: Token::FromConfig("test".to_string()),
            },
        };

        let text_from_config = r#"
vault:
    server: "127.0.0.1:8200"
    token:
        from_config: "test"
        "#;

        let de_from_config: Config = serde_yaml::from_str(text_from_config).unwrap();
        assert_eq!(de_from_config, from_config);
        // Just assert that it can be serialized, not about to do string comparison
        serde_yaml::to_string(&from_config).unwrap();
    }

    #[test]
    fn test_token_disk_parsing() {
        let from_disk = Config {
            vault: VaultConfig {
                namespace: None,
                server: "127.0.0.1:8200".to_string(),
                ca_certificate: None,
                token: Token::FromDisk(PathBuf::from("/token")),
            },
        };

        let text_from_disk = r#"
vault:
    server: "127.0.0.1:8200"
    token:
        from_disk: "/token"
        "#;

        let de_from_disk: Config = serde_yaml::from_str(text_from_disk).unwrap();
        assert_eq!(de_from_disk, from_disk);
        // Just assert that it can be serialized, not about to do string comparison
        serde_yaml::to_string(&from_disk).unwrap();
    }

    #[test]
    fn test_token_reading() {
        let temppath = libra_temppath::TempPath::new();
        temppath.create_as_file().unwrap();
        let mut file = File::create(temppath.path()).unwrap();
        file.write_all(b"disk_token").unwrap();

        let disk = Token::FromDisk(temppath.path().to_path_buf());
        assert_eq!("disk_token", disk.read_token().unwrap());

        let config = Token::FromConfig("config_token".to_string());
        assert_eq!("config_token", config.read_token().unwrap());
    }
}