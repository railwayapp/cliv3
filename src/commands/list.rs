use std::collections::{BTreeSet, HashSet};

use super::{queries::project, *};

/// List all projects in your Railway account
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    let configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project().ok();

    let vars = queries::user_projects::Variables {};

    let res = post_graphql::<queries::UserProjects, _>(
        &client,
        "https://backboard.railway.app/graphql/v2",
        vars,
    )
    .await?;

    let body = res.data.context("Failed to retrieve response body")?;

    let mut projects: Vec<_> = body
        .me
        .projects
        .edges
        .iter()
        .map(|project| &project.node)
        .collect();
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let teams: Vec<_> = body.me.teams.edges.iter().map(|team| &team.node).collect();

    println!("{}", "Personal".bold());
    for project in &projects {
        let project_name =
            if linked_project.is_some() && project.id == linked_project.unwrap().project {
                project.name.purple().bold()
            } else {
                project.name.white()
            };
        println!("  {}", project_name);
    }

    for team in teams {
        println!();
        println!("{}", team.name.bold());
        {
            let vars = queries::projects::Variables {
                team_id: Some(team.id.clone()),
            };

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

            for project in &projects {
                let project_name =
                    if linked_project.is_some() && project.id == linked_project.unwrap().project {
                        project.name.purple().bold()
                    } else {
                        project.name.white()
                    };
                println!("  {}", project_name);
            }
        }
    }

    Ok(())
}
