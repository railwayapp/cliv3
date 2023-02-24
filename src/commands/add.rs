use std::time::Duration;

use crate::consts::{PLUGINS, TICK_STRING};

use super::{queries::project_plugins::PluginType, *};

/// Add a new plugin to your project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let render_config = configs.get_render_config();

    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project()?;

    let vars = queries::project_plugins::Variables {
        id: linked_project.project.clone(),
    };

    let res =
        post_graphql::<queries::ProjectPlugins, _>(&client, configs.get_backboard(), vars).await?;

    let body = res.data.context("Failed to retrieve response body")?;

    let project_plugins: Vec<_> = body
        .project
        .plugins
        .edges
        .iter()
        .map(|p| plugin_enum_to_string(&p.node.name))
        .collect();

    let filtered_plugins: Vec<_> = PLUGINS
        .iter()
        .filter(|plugin| !project_plugins.contains(&plugin.to_string()))
        .collect();

    let selected = inquire::MultiSelect::new("Select plugins to add", filtered_plugins)
        .with_render_config(render_config)
        .prompt()?;

    for plugin in selected {
        let vars = mutations::plugin_create::Variables {
            project_id: linked_project.project.clone(),
            name: plugin.to_lowercase(),
        };
        let spinner = indicatif::ProgressBar::new_spinner()
            .with_style(
                indicatif::ProgressStyle::default_spinner()
                    .tick_chars(TICK_STRING)
                    .template("{spinner:.green} {msg}")?,
            )
            .with_message(format!("Creating {plugin}..."));
        spinner.enable_steady_tick(Duration::from_millis(100));

        post_graphql::<mutations::PluginCreate, _>(&client, configs.get_backboard(), vars).await?;

        spinner.finish_with_message(format!("Created {plugin}"));
    }

    Ok(())
}

fn plugin_enum_to_string(plugin: &PluginType) -> String {
    match plugin {
        PluginType::postgresql => "PostgreSQL".to_owned(),
        PluginType::mysql => "MySQL".to_owned(),
        PluginType::redis => "Redis".to_owned(),
        PluginType::mongodb => "MongoDB".to_owned(),
        PluginType::Other(other) => other.to_owned(),
    }
}
