use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use graphql_client::GraphQLQuery;
use gzp::{deflate::Gzip, ZBuilder};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressFinish, ProgressIterator, ProgressStyle};
use synchronized_writer::SynchronizedWriter;
use tar::Builder;
use tokio_stream::StreamExt;

use crate::{consts::TICK_STRING, entities::UpResponse, subscription::connect_subscription_client};

use super::*;

/// Upload and deploy project from the current directory
#[derive(Parser)]
pub struct Args {
    path: Option<PathBuf>,

    #[clap(short, long)]
    /// Don't attach to the log stream
    detach: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let configs = Configs::new()?;
    let hostname = configs.get_host();
    let client = GQLClient::new_authorized(&configs)?;
    let linked_project = configs.get_linked_project()?;

    let spinner = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::default_spinner()
                .tick_chars(TICK_STRING)
                .template("{spinner:.green} {msg:.cyan.bold}")?,
        )
        .with_message("Indexing".to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));
    let bytes = Vec::<u8>::new();
    let arc = Arc::new(Mutex::new(bytes));
    let mut parz = ZBuilder::<Gzip, _>::new()
        .num_threads(num_cpus::get())
        .from_writer(SynchronizedWriter::new(arc.clone()));
    {
        let mut archive = Builder::new(&mut parz);
        let mut builder = WalkBuilder::new(args.path.unwrap_or_else(|| ".".into()));
        let walker = builder.follow_links(true).hidden(false);
        let walked = walker.build().collect::<Vec<_>>();
        spinner.finish_with_message("Indexed");

        let pg = ProgressBar::new(walked.len() as u64)
            .with_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} {msg:.cyan.bold} [{bar:20}] {percent}% ")?
                    .progress_chars("=> ")
                    .tick_chars(TICK_STRING),
            )
            .with_message("Compressing")
            .with_finish(ProgressFinish::WithMessage("Compressed".into()));
        pg.enable_steady_tick(Duration::from_millis(100));

        for entry in walked.into_iter().progress_with(pg) {
            archive.append_path(entry?.path())?;
        }
    }
    parz.finish()?;

    let builder = client.post(format!(
        "https://backboard.{hostname}/project/{}/environment/{}/up",
        linked_project.project, linked_project.environment
    ));

    let spinner = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::default_spinner()
                .tick_chars(TICK_STRING)
                .template("{spinner:.green} {msg:.cyan.bold}")?,
        )
        .with_message("Uploading");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let body = arc.lock().unwrap().clone();
    let res = builder
        .header("Content-Type", "multipart/form-data")
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    let body = res.json::<UpResponse>().await?;
    spinner.finish_with_message("Uploaded");
    println!("  {}: {}", "Build Logs".green().bold(), body.logs_url);
    if args.detach {
        return Ok(());
    }

    let mut subscription_client = connect_subscription_client(&configs).await?;

    let vars = queries::deployments::Variables {
        project_id: linked_project.project.clone(),
    };

    let res =
        post_graphql::<queries::Deployments, _>(&client, configs.get_backboard(), vars).await?;

    let body = res.data.context("Failed to retrieve response body")?;

    let mut deployments: Vec<_> = body
        .project
        .deployments
        .edges
        .into_iter()
        .map(|deployment| deployment.node)
        .collect();
    deployments.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let latest_deployment = deployments.first().context("No deployments found")?;

    let vars = subscriptions::build_logs::Variables {
        deployment_id: latest_deployment.id.clone(),
        filter: Some(String::new()),
        limit: Some(500),
    };
    let query = subscriptions::BuildLogs::build_query(vars);
    let mut subscription = subscription_client
        .start::<subscriptions::BuildLogs>(&query)
        .await?;
    while let Some(log) = subscription.next().await {
        if let Some(log) = log {
            let log = log.data.context("Failed to retrieve log")?;
            for line in log.build_logs {
                println!("{}", line.message);
            }
        }
    }
    Ok(())
}
