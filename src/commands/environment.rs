use std::fmt::Display;

use super::{queries::project::ProjectProjectEnvironmentsEdgesNode, *};

/// Change the active environment
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let mut configs = Configs::new()?;
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

    let environments: Vec<_> = body
        .project
        .environments
        .edges
        .iter()
        .map(|env| Environment(&env.node))
        .collect();

    let environment = inquire::Select::new("Select an environment", environments)
        .with_render_config(configs.get_render_config())
        .prompt()?;

    configs.link_project(linked_project.project.clone(), environment.0.id.clone())?;
    configs.write()?;
    Ok(())
}

#[derive(Debug, Clone)]
struct Environment<'a>(&'a ProjectProjectEnvironmentsEdgesNode);

impl<'a> Display for Environment<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}
