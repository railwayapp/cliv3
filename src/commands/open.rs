use super::*;

/// Open your project dashboard
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args, json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let linked_project = configs.get_linked_project()?;
    ::open::that(format!(
        "https://railway.app/project/{}",
        linked_project.project
    ))?;
    Ok(())
}
