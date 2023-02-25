use std::fmt::Display;

use anyhow::bail;

use crate::{consts::NO_SERVICE_LINKED, table::Table};

use super::{
    queries::project::{PluginType, ProjectProjectPluginsEdgesNode},
    *,
};

/// Show variables for active environment
#[derive(Parser)]
pub struct Args {
    /// Show variables for a specific environment
    environment: Option<String>,
    /// Show variables for a plugin
    #[clap(short, long)]
    plugin: bool,

    /// Show variables in KV format
    #[clap(short, long)]
    kv: bool,
}

pub async fn command(args: Args, json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project()?;

    let vars = queries::project::Variables {
        id: linked_project.project.to_owned(),
    };

    let res = post_graphql::<queries::Project, _>(&client, configs.get_backboard(), vars).await?;

    let body = res.data.context("Failed to retrieve response body")?;
    let plugins: Vec<_> = body
        .project
        .plugins
        .edges
        .iter()
        .map(|plugin| Plugin(&plugin.node))
        .collect();

    let environment = if let Some(environment) = args.environment.clone() {
        let envs = &body
            .project
            .environments
            .edges
            .iter()
            .map(|env| env.node.clone())
            .find(|env| env.name == environment);

        if envs.is_none() {
            bail!("Environment not found");
        }

        envs.clone().unwrap().id
    } else {
        linked_project.environment.clone()
    };

    let (vars, name) = if args.plugin {
        if plugins.is_empty() {
            bail!("No plugins found");
        }
        let plugin = prompt_plugin(plugins)?;
        (
            queries::variables::Variables {
                environment_id: environment,
                project_id: linked_project.project.clone(),
                service_id: None,
                plugin_id: Some(plugin.0.id.clone()),
            },
            format!("{plugin}"),
        )
    } else if let Some(ref service) = linked_project.service {
        let service_name = body
            .project
            .services
            .edges
            .iter()
            .find(|edge| edge.node.id == *service)
            .context("Service not found")?;
        (
            queries::variables::Variables {
                environment_id: environment,
                project_id: linked_project.project.clone(),
                service_id: Some(service.clone()),
                plugin_id: None,
            },
            service_name.node.name.clone(),
        )
    } else {
        if plugins.is_empty() {
            bail!(NO_SERVICE_LINKED);
        }
        let plugin = prompt_plugin(plugins)?;
        (
            queries::variables::Variables {
                environment_id: environment,
                project_id: linked_project.project.clone(),
                service_id: None,
                plugin_id: Some(plugin.0.id.clone()),
            },
            format!("{plugin}"),
        )
    };

    let res = post_graphql::<queries::Variables, _>(&client, configs.get_backboard(), vars).await?;

    let body = res.data.context("Failed to retrieve response body")?;

    if body.variables.is_empty() {
        eprintln!("No variables found");
        return Ok(());
    }

    if args.kv {
        for (key, value) in body.variables {
            println!("{}={}", key, value);
        }
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&body.variables)?);
        return Ok(());
    }

    let table = Table::new(name, body.variables);
    table.print()?;

    Ok(())
}

fn prompt_plugin(plugins: Vec<Plugin>) -> Result<Plugin> {
    let configs = Configs::new()?;
    let plugin = inquire::Select::new("Select a plugin", plugins)
        .with_render_config(configs.get_render_config())
        .prompt()?;

    Ok(plugin)
}

struct Plugin<'a>(&'a ProjectProjectPluginsEdgesNode);

impl<'a> Display for Plugin<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self.0.name {
                PluginType::mongodb => "MongoDB",
                PluginType::mysql => "MySQL",
                PluginType::postgresql => "PostgreSQL",
                PluginType::redis => "Redis",
                PluginType::Other(plugin) => plugin,
            }
        )
    }
}
