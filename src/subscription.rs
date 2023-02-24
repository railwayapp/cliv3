use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
};

use anyhow::{bail, Result};
use futures::SinkExt;
use graphql_client::GraphQLQuery;
use hyper::http::HeaderValue;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{
    broadcast::{self, Sender},
    mpsc,
};
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};
use uuid::Uuid;

use crate::commands::Configs;

/// Subscription GraphQL response, returned from an active stream.
pub type BoxedSubscription<T> = Pin<
    Box<
        dyn Stream<Item = Option<graphql_client::Response<<T as GraphQLQuery>::ResponseData>>>
            + Send
            + Sync,
    >,
>;

/// Payload contains the raw data received back from a GraphQL subscription. At the point
/// of receiving data, the only known fields are { id, type }; what's contained inside the
/// `payload` field is unknown until we attempt to deserialize it against a generated
/// GraphQLQuery::ResponseData later.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Payload {
    id: Option<Uuid>,
    #[serde(rename = "type")]
    payload_type: String,
    payload: serde_json::Value,
}

impl Payload {
    /// Returns an "init" payload to confirm the connection to the server.
    pub fn init(id: Uuid) -> Self {
        Self {
            id: Some(id),
            payload_type: "connection_init".to_owned(),
            payload: json!({}),
        }
    }

    /// Returns a "start" payload necessary for starting a new subscription.
    pub fn start<T: GraphQLQuery + Send + Sync>(
        id: Uuid,
        payload: &graphql_client::QueryBody<T::Variables>,
    ) -> Self {
        Self {
            id: Some(id),
            payload_type: "subscribe".to_owned(),
            payload: json!(payload),
        }
    }

    /// Attempts to return a definitive ResponseData on the `payload` field, matched against
    /// a generated `GraphQLQuery`.
    fn response<T: GraphQLQuery + Send + Sync>(
        &self,
    ) -> Option<graphql_client::Response<T::ResponseData>> {
        serde_json::from_value::<graphql_client::Response<T::ResponseData>>(self.payload.clone())
            .ok()
    }
}

/// A single `SubscriptionClient` enables subscription multiplexing.
#[derive(Debug)]
pub struct SubscriptionClient {
    tx: mpsc::UnboundedSender<String>,
    rx: mpsc::UnboundedReceiver<String>,
    subscriptions: Arc<Mutex<HashMap<Uuid, Sender<Payload>>>>,
}

impl SubscriptionClient {
    /// Create a new subscription client. `tx` is a channel for sending `Payload`s to the
    /// GraphQL server; `rx` is a channel for `Payload` back.
    fn new(tx: mpsc::UnboundedSender<String>, mut rx: mpsc::UnboundedReceiver<String>) -> Self {
        // Oneshot channel for cancelling the listener if SubscriptionClient is dropped
        let (tx_out, rx_out) = mpsc::unbounded_channel::<String>();
        let subscriptions = Arc::new(Mutex::new(HashMap::new()));
        let subscriptions_clone = Arc::clone(&subscriptions);

        // Spawn a handler for shutdown, and relaying received `Payload`s back to the relevant
        // subscription.
        tokio::spawn(async move {
            loop {
                // Handle receiving payloads back _from_ the server
                let message = rx.recv().await;
                {
                    match message {
                        Some(p) => {
                            tx_out.send(p.clone()).unwrap();
                            let subscriptions = subscriptions_clone.lock().unwrap();
                            if let Ok(p) = serde_json::from_str::<Payload>(&p) {
                                let s: Option<&Sender<Payload>> =
                                    p.id.map(|id| subscriptions.get::<Uuid>(&id)).flatten();
                                if let Some(s) = s {
                                    let _ = s.send(p);
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
            // }
        });

        Self {
            tx,
            rx: rx_out,
            subscriptions,
        }
    }

    /// Start a new subscription request.
    pub async fn start<T: GraphQLQuery + Send + Sync>(
        &mut self,
        request_body: &graphql_client::QueryBody<T::Variables>,
    ) -> Result<BoxedSubscription<T>>
    where
        T: GraphQLQuery + Send + Sync,
        <T as GraphQLQuery>::ResponseData: Unpin + Send + Sync + 'static,
    {
        // Generate a unique ID for the subscription. Subscriptions can be multiplexed
        // over a single connection, so we'll keep a copy of this against the client to
        // handling routing responses back to the relevant subscriber.
        let id = Uuid::new_v4();

        let (tx, rx) = broadcast::channel::<Payload>(100);

        self.subscriptions.lock().unwrap().insert(id, tx);

        // Initialize the connection with the relevant control messages.
        let _ = self.tx.send(serde_json::to_string(&Payload::init(id))?);
        // TODO: actually wait on ack
        if let Some(ack) = self.rx.recv().await {
            let ack: Payload = serde_json::from_str(&ack)?;
            if ack.payload_type != "connection_ack" {
                bail!("Expected connection_ack, got {:?}", ack);
            }
            println!()
        }
        let _ = self.tx.send(serde_json::to_string(&Payload::start::<T>(
            id,
            request_body,
        ))?);

        Ok(Box::pin(
            BroadcastStream::new(rx)
                .filter(Result::is_ok)
                .map(|p| p.unwrap().response::<T>()),
        ))
    }
}

/// Connect to a new WebSocket GraphQL server endpoint, and return a `SubscriptionClient`.
/// This method will a) connect to a ws(s):// endpoint, and perform the initial handshake, and b)
/// set up channel forwarding to expose just the returned `Payload`s to the client.
pub async fn connect_subscription_client(configs: &Configs) -> Result<SubscriptionClient> {
    let Some(token) = configs.root_config.user.token.clone() else {
      bail!("Unauthorized. Please login with `railway login`")
    };
    let bearer = format!("Bearer {}", token);
    let hostname = configs.get_host();
    let mut request = format!("wss://backboard.{hostname}/graphql/v2").into_client_request()?;

    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws")?,
    );
    request
        .headers_mut()
        .insert("Authorization", HeaderValue::from_str(&bearer)?);
    // let origin = format!("https://{hostname}");
    // request
    //     .headers_mut()
    //     .insert("Origin", HeaderValue::from_str(origin.as_str())?);
    // request.headers_mut().insert(
    //     "accept-encoding",
    //     HeaderValue::from_static("gzip, deflate, br"),
    // );
    // request.headers_mut().insert(
    //     "sec-websocket-extensions",
    //     HeaderValue::from_static("permessage-deflate; client_max_window_bits"),
    // );
    // request.headers_mut().insert(
    //     "user-agent",
    //     HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36"),
    // );

    let (ws, _) = connect_async(request).await?;
    let (mut ws_tx, mut ws_rx) = futures::StreamExt::split(ws);

    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<String>();
    let (recv_tx, recv_rx) = mpsc::unbounded_channel::<String>();

    // Forwarded received messages back upstream to the GraphQL server
    tokio::spawn(async move {
        while let Some(p) = send_rx.recv().await {
            let _ = ws_tx.send(Message::Text(p)).await;
        }
    });

    // Forward received messages to the receiver channel.
    tokio::spawn(async move {
        while let Some(Ok(Message::Text(m))) = ws_rx.next().await {
            let _ = recv_tx.send(m);
            // if let Ok(p) = serde_json::from_str::<Payload>(&m) {
            //     let _ = recv_tx.send(p);
            // }
        }
    });

    Ok(SubscriptionClient::new(send_tx, recv_rx))
}
