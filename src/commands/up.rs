use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use gzp::{deflate::Gzip, ZBuilder};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use synchronized_writer::SynchronizedWriter;
use tar::Builder;

use crate::{consts::TICK_STRING, entities::UpResponse};

use super::*;

/// Upload and deploy project from the current directory
#[derive(Parser)]
pub struct Args {
    path: Option<PathBuf>,

    #[clap(short, long)]
    /// Don't attach to the log stream
    detach: bool,
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

    let spinner = indicatif::ProgressBar::new_spinner()
        .with_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_chars(TICK_STRING)
                .template("{spinner:.green} {msg:.cyan.bold}")?,
        )
        .with_message(format!("Indexing"));
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
            .with_finish(indicatif::ProgressFinish::WithMessage("Compressed".into()));
        pg.enable_steady_tick(Duration::from_millis(100));

        for entry in walked.into_iter().progress_with(pg) {
            archive.append_path(entry?.path())?;
        }
    }
    parz.finish()?;
    let client = GQLClient::new_authorized(&configs)?;

    let builder = client.post(format!(
        "https://backboard.railway.app/project/{}/environment/{}/up",
        linked_project.project, linked_project.environment
    ));

    let spinner = indicatif::ProgressBar::new_spinner()
        .with_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_chars(TICK_STRING)
                .template("{spinner:.green} {msg:.cyan.bold}")?,
        )
        .with_message("Uploading");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let res = builder
        .header("Content-Type", "multipart/form-data")
        .body(arc.lock().unwrap().clone())
        .send()
        .await?
        .error_for_status()?;

    let body = res.json::<UpResponse>().await?;
    spinner.finish_with_message("Uploaded");
    println!("  {}: {}", "Build Logs".green().bold(), body.logs_url);
    if args.detach {
        return Ok(());
    }
    Ok(())
}
