use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use inquire::ui::{Attributes, RenderConfig, StyleSheet, Styled};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RailwayProject {
    pub project_path: String,
    pub name: Option<String>,
    pub project: String,
    pub environment: String,
    pub environment_name: Option<String>,
    pub service: Option<String>,
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

pub enum Environment {
    Production,
    Staging,
    Dev,
}

impl Configs {
    pub fn new() -> Result<Self> {
        let environment = Self::get_environment_id();
        let root_config_partial_path = match environment {
            Environment::Production => ".railway/config.json",
            Environment::Staging => ".railway/config-staging.json",
            Environment::Dev => ".railway/config-dev.json",
        };

        let home_dir = dirs::home_dir().context("Unable to get home directory")?;
        let root_config_path = std::path::Path::new(&home_dir).join(root_config_partial_path);

        if let Ok(mut file) = File::open(&root_config_path) {
            let mut serialized_config = vec![];
            file.read_to_end(&mut serialized_config)?;

            let root_config: RailwayConfig = serde_json::from_slice(&serialized_config)?;

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

    pub fn get_environment_id() -> Environment {
        match std::env::var("RAILWAY_ENV")
            .map(|env| env.to_lowercase())
            .as_deref()
        {
            Ok("production") => Environment::Production,
            Ok("staging") => Environment::Staging,
            Ok("dev") => Environment::Dev,
            _ => Environment::Production,
        }
    }

    pub fn get_host(&self) -> &'static str {
        match Self::get_environment_id() {
            Environment::Production => "railway.app",
            Environment::Staging => "railway-staging.app",
            Environment::Dev => "railway-develop.app",
        }
    }

    pub fn get_backboard(&self) -> String {
        return format!("https://backboard.{}/graphql/v2", self.get_host());
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

    pub fn link_project(
        &mut self,
        project_id: String,
        name: Option<String>,
        environment_id: String,
        environment_name: Option<String>,
    ) -> Result<()> {
        let path = Self::get_current_working_directory()?;
        let project = RailwayProject {
            project_path: path.clone(),
            name,
            project: project_id,
            environment: environment_id,
            environment_name,
            service: None,
        };
        self.root_config.projects.insert(path, project);
        Ok(())
    }

    pub fn link_service(&mut self, service_id: String) -> Result<()> {
        let linked_project = self.get_linked_project_mut()?;
        linked_project.service = Some(service_id);
        Ok(())
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

    pub fn unlink_service(&mut self) -> Result<()> {
        let linked_project = self.get_linked_project_mut()?;
        linked_project.service = None;
        Ok(())
    }

    pub fn get_render_config(&self) -> RenderConfig {
        RenderConfig::default_colored()
            .with_help_message(
                StyleSheet::new()
                    .with_fg(inquire::ui::Color::LightMagenta)
                    .with_attr(Attributes::BOLD),
            )
            .with_answer(
                StyleSheet::new()
                    .with_fg(inquire::ui::Color::LightCyan)
                    .with_attr(Attributes::BOLD),
            )
            .with_prompt_prefix(
                Styled::new("?").with_style_sheet(
                    StyleSheet::new()
                        .with_fg(inquire::ui::Color::LightCyan)
                        .with_attr(Attributes::BOLD),
                ),
            )
    }

    pub fn write(&self) -> Result<()> {
        create_dir_all(self.root_config_path.parent().unwrap())?;
        let mut file = File::create(&self.root_config_path)?;
        let serialized_config = serde_json::to_vec_pretty(&self.root_config)?;
        file.write_all(serialized_config.as_slice())?;
        file.sync_all()?;
        Ok(())
    }
}
