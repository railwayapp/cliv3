use std::{net::SocketAddr, time::Duration};

use crate::consts::{ABORTED_BY_USER, TICK_STRING};

use super::*;
use anyhow::bail;
use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

/// Login to your Railway account
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let mut config = Configs::new()?;
    let render_config = config.get_render_config();

    let confirm = inquire::Confirm::new("Open the browser")
        .with_default(true)
        .with_render_config(render_config)
        .prompt()?;

    if !confirm {
        bail!(ABORTED_BY_USER);
    }

    let port = rand::thread_rng().gen_range(50000..60000);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = TcpListener::bind(addr).await?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1);
    let hello = move |req: Request<hyper::body::Incoming>| {
        let tx = tx.clone();
        async move {
            if req.method() == hyper::Method::GET {
                let mut pairs = req.uri().query().context("No query")?.split('&');

                let token = pairs
                    .next()
                    .context("No token")?
                    .split('=')
                    .nth(1)
                    .context("No token")?
                    .to_owned();

                tx.send(token).await?;
                let res = LoginResponse {
                    status: "Ok".to_owned(),
                    error: "".to_owned(),
                };
                let res_json = serde_json::to_string(&res)?;
                let mut response = Response::new(Full::from(res_json));
                response.headers_mut().insert(
                    "Content-Type",
                    hyper::header::HeaderValue::from_static("application/json"),
                );
                response.headers_mut().insert(
                    "Access-Control-Allow-Origin",
                    hyper::header::HeaderValue::from_static("https://railway.app"),
                );
                Ok::<Response<Full<Bytes>>, anyhow::Error>(response)
            } else {
                let mut response = Response::default();
                response.headers_mut().insert(
                    "Access-Control-Allow-Methods",
                    hyper::header::HeaderValue::from_static("GET, HEAD, PUT, PATCH, POST, DELETE"),
                );
                response.headers_mut().insert(
                    "Access-Control-Allow-Headers",
                    hyper::header::HeaderValue::from_static("*"),
                );
                response.headers_mut().insert(
                    "Access-Control-Allow-Origin",
                    hyper::header::HeaderValue::from_static("https://railway.app"),
                );
                response.headers_mut().insert(
                    "Content-Length",
                    hyper::header::HeaderValue::from_static("0"),
                );
                *response.status_mut() = hyper::StatusCode::NO_CONTENT;
                Ok::<Response<Full<Bytes>>, anyhow::Error>(response)
            }
        }
    };

    ::open::that(generate_cli_login_url(port)?)?;
    let spinner = indicatif::ProgressBar::new_spinner()
        .with_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_chars(TICK_STRING)
                .template("{spinner:.green} {msg}")?,
        )
        .with_message("Waiting for login...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let (stream, _) = listener.accept().await?;

    // Intentionally not awaiting this task, so that we exit after a single request
    tokio::task::spawn(async move {
        http1::Builder::new()
            .serve_connection(stream, service_fn(hello))
            .await?;
        Ok::<_, anyhow::Error>(())
    });

    let token = rx.recv().await.context("No token received")?;
    config.root_config.user.token = Some(token);
    config.write()?;

    let client = GQLClient::new_authorized(&config)?;
    let vars = queries::user_meta::Variables {};

    let res = post_graphql::<queries::UserMeta, _>(
        &client,
        "https://backboard.railway.app/graphql/v2",
        vars,
    )
    .await?;
    let me = res.data.context("No data")?.me;

    spinner.finish_and_clear();
    println!(
        "Logged in as {} ({})",
        me.name.context("No name")?.bold(),
        me.email
    );

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    status: String,
    error: String,
}

fn get_random_numeric_code(length: usize) -> String {
    let mut rng = rand::thread_rng();

    std::iter::from_fn(|| rng.gen_range(0..10).to_string().chars().next())
        .take(length)
        .collect()
}

fn generate_login_payload(port: u16) -> Result<String> {
    let code = get_random_numeric_code(32);
    let hostname_os = hostname::get()?;
    let hostname = hostname_os.to_str().context("Invalid hostname")?;
    let payload = format!("port={port}&code={code}&hostname={hostname}");
    Ok(payload)
}

fn generate_cli_login_url(port: u16) -> Result<String> {
    use base64::{
        alphabet::URL_SAFE,
        engine::{GeneralPurpose, GeneralPurposeConfig},
        Engine,
    };
    let payload = generate_login_payload(port)?;

    let engine = GeneralPurpose::new(&URL_SAFE, GeneralPurposeConfig::new());
    let encoded_payload = engine.encode(payload.as_bytes());

    let url = format!("https://railway.app/cli-login?d={encoded_payload}");
    Ok(url)
}
