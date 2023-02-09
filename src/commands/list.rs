use std::collections::{BTreeSet, HashSet};

use super::{queries::project, *};

/// List all projects in your Railway account
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    let configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project()?;

    let vars = queries::projects::Variables {};

    let res = post_graphql::<queries::Projects, _>(
        &client,
        "https://backboard.railway.app/graphql/v2",
        vars,
    )
    .await?;

    let body = res.data.context("Failed to retrieve response body")?;

    let mut projects: Vec<_> = body
        .projects
        .edges
        .iter()
        .map(|project| &project.node)
        .collect();
    projects.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));

    let mut teams = BTreeSet::<String>::new();

    for project in &projects {
        if let Some(team) = &project.team {
            teams.insert(team.name.clone());
        } else {
            teams.insert("Personal".to_string());
        }
    }

    for team in teams {
        println!("{}", team.bold());
        for project in &projects {
            let project_name = if project.id == linked_project.project {
                project.name.purple().bold()
            } else {
                project.name.white()
            };

            if let Some(project_team) = &project.team {
                if project_team.name == team {
                    println!("  {}", project_name);
                }
            } else if project.team.is_none() && team == "Personal" {
                println!("  {}", project_name);
            }
        }
    }

    Ok(())
}
