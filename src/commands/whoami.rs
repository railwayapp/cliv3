use super::*;

/// Get the current logged in user
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    let configs = Configs::new()?;
    let client = GQLClient::new_authorized(&configs)?;
    let vars = queries::user_meta::Variables {};

    let res = post_graphql::<queries::UserMeta, _>(
        &client,
        "https://backboard.railway.app/graphql/v2",
        vars,
    )
    .await?;
    let me = res.data.context("No data")?.me;

    println!(
        "Logged in as {} ({})",
        me.name.context("No name")?.bold(),
        me.email
    );

    Ok(())
}
