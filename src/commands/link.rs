use std::fmt::Display;

use crate::commands::queries::user_projects::UserProjectsMeTeamsEdgesNode;

use super::{
    queries::{
        projects::{ProjectsProjectsEdgesNode, ProjectsProjectsEdgesNodeEnvironmentsEdgesNode},
        user_projects::{
            UserProjectsMeProjectsEdgesNode, UserProjectsMeProjectsEdgesNodeEnvironmentsEdgesNode,
        },
    },
    *,
};

/// Associate existing project with current directory, may specify projectId as an argument
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    let mut configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;

    let vars = queries::user_projects::Variables {};

    let res = post_graphql::<queries::UserProjects, _>(
        &client,
        "https://backboard.railway.app/graphql/v2",
        vars,
    )
    .await?;

    let body = res.data.context("Failed to retrieve response body")?;

    let mut personal_projects: Vec<_> = body
        .me
        .projects
        .edges
        .iter()
        .map(|project| &project.node)
        .collect();
    personal_projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let personal_project_names = personal_projects
        .iter()
        .map(|project| PersonalProject(project))
        .collect::<Vec<_>>();

    let teams: Vec<_> = body.me.teams.edges.iter().map(|team| &team.node).collect();

    if teams.is_empty() {
        let (project, environment) = prompt_personal_projects(personal_project_names)?;
        configs.link_project(project.0.id.clone(), environment.0.id.clone())?;
        return Ok(());
    }

    let mut team_names = teams
        .iter()
        .map(|team| Team::Team(team))
        .collect::<Vec<_>>();
    team_names.insert(0, Team::Personal);

    let team = inquire::Select::new("Select a team", team_names)
        .with_render_config(configs.get_render_config())
        .prompt()?;
    match team {
        Team::Personal => {
            let (project, environment) = prompt_personal_projects(personal_project_names)?;
            configs.link_project(project.0.id.clone(), environment.0.id.clone())?;
        }
        Team::Team(team) => {
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
            projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

            let project_names = projects
                .iter()
                .map(|project| Project(project))
                .collect::<Vec<_>>();
            let (project, environment) = prompt_team_projects(project_names)?;
            configs.link_project(project.0.id.clone(), environment.0.id.clone())?;
        }
    }

    configs.write()?;

    Ok(())
}

fn prompt_team_projects<'a>(project_names: Vec<Project>) -> Result<(Project, Environment)> {
    let configs = Configs::new()?;

    let project = inquire::Select::new("Select a project", project_names)
        .with_render_config(configs.get_render_config())
        .prompt()?;
    let environments = project
        .0
        .environments
        .edges
        .iter()
        .map(|env| Environment(&env.node))
        .collect();
    let environment = inquire::Select::new("Select an environment", environments)
        .with_render_config(configs.get_render_config())
        .prompt()?;
    return Ok((project, environment));
}
fn prompt_personal_projects<'a>(
    personal_project_names: Vec<PersonalProject>,
) -> Result<(PersonalProject, PersonalEnvironment)> {
    let configs = Configs::new()?;

    let project = inquire::Select::new("Select a project", personal_project_names)
        .with_render_config(configs.get_render_config())
        .prompt()?;
    let environments = project
        .0
        .environments
        .edges
        .iter()
        .map(|env| PersonalEnvironment(&env.node))
        .collect();
    let environment = inquire::Select::new("Select an environment", environments)
        .with_render_config(configs.get_render_config())
        .prompt()?;
    return Ok((project, environment));
}

#[derive(Debug, Clone)]
struct PersonalProject<'a>(&'a UserProjectsMeProjectsEdgesNode);

impl<'a> Display for PersonalProject<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

#[derive(Debug, Clone)]
struct PersonalEnvironment<'a>(&'a UserProjectsMeProjectsEdgesNodeEnvironmentsEdgesNode);

impl<'a> Display for PersonalEnvironment<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

#[derive(Debug, Clone)]
struct Project<'a>(&'a ProjectsProjectsEdgesNode);

impl<'a> Display for Project<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

#[derive(Debug, Clone)]
struct Environment<'a>(&'a ProjectsProjectsEdgesNodeEnvironmentsEdgesNode);

impl<'a> Display for Environment<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

#[derive(Debug, Clone)]
enum Team<'a> {
    Team(&'a UserProjectsMeTeamsEdgesNode),
    Personal,
}

impl<'a> Display for Team<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Team::Team(team) => write!(f, "{}", team.name),
            Team::Personal => write!(f, "{}", "Personal".bold()),
        }
    }
}
