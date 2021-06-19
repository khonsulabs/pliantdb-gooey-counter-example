use std::path::Path;

use pliantdb::{
    core::{connection::ServerConnection, kv::Kv, permissions::Permissions, pubsub::PubSub, Error},
    server::{Backend, Configuration, CustomServer},
};
use shared::{
    ExampleApi, IncrementCounterHandler, Request, RequestDispatcher, Response,
    COUNTER_CHANGED_TOPIC, DATABASE_NAME,
};

/// The server's main entrypoint.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open a `PliantDb` server at the given path, allowing all actions to be
    // done over the network connections.
    let server = CustomServer::<Example>::open(Path::new("server-data.pliantdb"), Configuration {
        default_permissions: Permissions::allow_all(),
        ..Configuration::default()
    })
    .await?;
    // Sets the dispatcher for custom API requests.
    server
        .set_custom_api_dispatcher(ApiDispatcher {
            server: server.clone(),
        })
        .await;
    server.register_schema::<()>().await?;
    // Create the database if it doesn't exist.
    match server.create_database::<()>(DATABASE_NAME).await {
        Ok(_) | Err(Error::DatabaseNameAlreadyTaken(_)) => {}
        Err(other) => anyhow::bail!(other),
    }
    // Start listening for websockets. This does not return until the server
    // shuts down. If you want to listen for multiple types of traffic, you will
    // need to spawn the tasks.
    server.listen_for_websockets_on("127.0.0.1:8081").await?;

    Ok(())
}

/// The example database `Backend`.
#[derive(Debug)]
enum Example {}
impl Backend for Example {
    type CustomApi = ExampleApi;
    type CustomApiDispatcher = ApiDispatcher;
}

/// The dispatcher for API requests.
#[derive(Debug, actionable::Dispatcher)]
#[dispatcher(input = Request)]
struct ApiDispatcher {
    server: CustomServer<Example>,
}

impl RequestDispatcher for ApiDispatcher {
    type Error = anyhow::Error;
    type Output = Response;
}

#[actionable::async_trait]
impl IncrementCounterHandler for ApiDispatcher {
    /// Increments the counter, and publishes a message with the new value.
    async fn handle(&self, _permissions: &actionable::Permissions) -> anyhow::Result<Response> {
        let db = self.server.database::<()>("counter").await?;

        // TODO implement increment. This has a race condition and should be implemented
        // with an atomic operation.
        let mut current_value: u64 = db.get_key("current-count").await?.unwrap_or_default();
        current_value += 1;
        db.set_key("current-count", &current_value).await?;

        db.publish(COUNTER_CHANGED_TOPIC, &current_value).await?;
        Ok(Response::CounterIncremented(current_value))
    }
}
