use std::fs::read_to_string;
use std::path::PathBuf;
use std::process::Command;
use toml::{from_str, value::Value};

#[derive(Debug, Clone)]
pub struct Metadata {
    pub id: String,
    pub bin: String,
    pub name: String,
    pub version: String,
    pub targetdir: PathBuf,
}

impl Metadata {
    pub fn from(path: &str) -> Option<Metadata> {
        let toml_str = read_to_string("Cargo.toml").expect("Error reading Cargo.toml!");
        let meta: Value = toml::from_str(&toml_str).expect("Error parsing Cargo.toml!");
        // println!("{:#?}", meta);

        let package = &meta.get("package")?;
        let metadata = &package.get("metadata")?;

        let bin = package.get("name")?.as_str()?.to_string();
        let version = package.get("version")?.as_str()?.to_string();

        let id = metadata.get("pkg")?.get("id")?.as_str()?.to_string();
        let name = metadata.get("pkg")?.get("name")?.as_str()?.to_string();

        const DEFAULT_TARGET_DIR: &str = "./target";

        let targetdir = meta
            .get("build")
            .unwrap_or(&Value::String(DEFAULT_TARGET_DIR.to_string()))
            .get("target-dir")
            .unwrap_or(&Value::String(DEFAULT_TARGET_DIR.to_string()))
            .as_str()?
            .to_string();

        Some(Self {
            id,
            bin,
            name,
            version,
            targetdir: PathBuf::from(targetdir),
        })
    }
}
