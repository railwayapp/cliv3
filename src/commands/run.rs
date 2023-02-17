use std::collections::BTreeMap;

use super::*;

/// Run a local command using variables from the active environment
#[derive(Debug, Parser)]
pub struct Args {
    /// Service to pull variables from (defaults to linked service)
    #[clap(short, long)]
    service: Option<String>,

    /// Command to run
    command: String,

    /// Args to pass to the command
    #[clap(raw = true)]
    args: Vec<String>,
}

pub async fn command(args: Args, json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project()?;

    let vars = queries::project::Variables {
        id: linked_project.project.to_owned(),
    };

    let res = post_graphql::<queries::Project, _>(
        &client,
        "https://backboard.railway.app/graphql/v2",
        vars,
    )
    .await?;

    let body = res.data.context("Failed to retrieve response body")?;
    let mut all_variables = BTreeMap::<String, String>::new();

    let plugins: Vec<_> = body
        .project
        .plugins
        .edges
        .iter()
        .map(|plugin| &plugin.node)
        .collect();

    for plugin in plugins {
        let vars = queries::variables::Variables {
            environment_id: linked_project.environment.clone(),
            project_id: linked_project.project.clone(),
            service_id: None,
            plugin_id: Some(plugin.id.clone()),
        };

        let res = post_graphql::<queries::Variables, _>(
            &client,
            "https://backboard.railway.app/graphql/v2",
            vars,
        )
        .await?;

        let mut body = res.data.context("Failed to retrieve response body")?;

        if body.variables.is_empty() {
            continue;
        }

        all_variables.append(&mut body.variables);
    }
    if linked_project.service.is_some() {
        let vars = queries::variables::Variables {
            environment_id: linked_project.environment.clone(),
            project_id: linked_project.project.clone(),
            service_id: linked_project.service.clone(),
            plugin_id: None,
        };

        let res = post_graphql::<queries::Variables, _>(
            &client,
            "https://backboard.railway.app/graphql/v2",
            vars,
        )
        .await?;

        let mut body = res.data.context("Failed to retrieve response body")?;

        all_variables.append(&mut body.variables);
    } else {
        println!("No service linked, skipping service variables");
    }

    tokio::process::Command::new(args.command)
        .args(args.args)
        .envs(all_variables)
        .spawn()
        .context("Failed to spawn command")?
        .wait()
        .await
        .context("Failed to wait for command")?;
    Ok(())
}
