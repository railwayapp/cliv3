use super::*;

/// Open your project dashboard
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let hostname = configs.get_host();
    let linked_project = configs.get_linked_project()?;
    ::open::that(format!(
        "https://{hostname}/project/{}",
        linked_project.project
    ))?;
    Ok(())
}
