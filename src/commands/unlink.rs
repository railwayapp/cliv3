use anyhow::bail;

use super::*;

/// Disassociate project from current directory
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
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

    println!("Linked to {}", body.project.name.bold());
    let confirmed = inquire::Confirm::new("Are you sure you want to unlink this project?")
        .with_render_config(configs.get_render_config())
        .with_default(true)
        .prompt()?;
    if !confirmed {
        bail!("Aborted by user");
    }
    configs.unlink_project()?;
    configs.write()?;
    Ok(())
}
