use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RailwayProject {
    pub project_path: String,
    pub project: String,
    pub environment: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RailwayUser {
    pub token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RailwayConfig {
    pub projects: BTreeMap<String, RailwayProject>,
    pub user: RailwayUser,
}

#[derive(Debug)]
pub struct Configs {
    pub root_config: RailwayConfig,
    root_config_path: PathBuf,
}

impl Configs {
    pub fn new() -> Result<Self> {
        let root_config_partial_path = ".railway/config.json";

        let home_dir = dirs::home_dir().context("Unable to get home directory")?;
        let root_config_path = std::path::Path::new(&home_dir).join(root_config_partial_path);

        if let Ok(mut file) = File::open(&root_config_path) {
            let mut serialized_config = vec![];
            file.read_to_end(&mut serialized_config)?;

            let mut root_config: RailwayConfig = serde_json::from_slice(&serialized_config)?;

            let config = Self {
                root_config,
                root_config_path,
            };

            config.write()?;

            return Ok(config);
        }

        Ok(Self {
            root_config_path,
            root_config: RailwayConfig {
                projects: BTreeMap::new(),
                user: RailwayUser { token: None },
            },
        })
    }

    pub fn get_railway_token() -> Option<String> {
        std::env::var("RAILWAY_TOKEN").ok()
    }

    pub fn get_current_working_directory() -> Result<String> {
        let current_dir = std::env::current_dir()?;
        let path = current_dir
            .to_str()
            .context("Unable to get current working directory")?;
        Ok(path.into())
    }

    pub fn get_linked_project(&self) -> Result<&RailwayProject> {
        let path = Self::get_current_working_directory()?;
        let project = self
            .root_config
            .projects
            .get(&path)
            .context("Project not found! Run `railway link` to link to a project")?;
        Ok(project)
    }

    pub fn get_linked_project_mut(&mut self) -> Result<&mut RailwayProject> {
        let path = Self::get_current_working_directory()?;
        let project = self
            .root_config
            .projects
            .get_mut(&path)
            .context("Project not found! Run `railway link` to link to a project")?;
        Ok(project)
    }

    pub fn unlink_project(&mut self) -> Result<RailwayProject> {
        let path = Self::get_current_working_directory()?;
        let project = self
            .root_config
            .projects
            .remove(&path)
            .context("Project not found! Run `railway link` to link to a project")?;
        Ok(project)
    }

    pub fn write(&self) -> Result<()> {
        create_dir_all(self.root_config_path.parent().unwrap())?;
        let mut file = File::create(&self.root_config_path)?;
        let serialized_config = serde_json::to_vec_pretty(&self.root_config)?;
        file.write_all(&mut serialized_config.as_slice())?;
        file.sync_all()?;
        Ok(())
    }
}
