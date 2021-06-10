use std::path::Path;

use pliantdb::{
    core::{backend::Backend, connection::ServerConnection, kv::Kv, pubsub::PubSub},
    server::{Configuration, CustomServer},
};
use shared::{ExampleApi, IncrementCounterHandler, Request, RequestDispatcher, Response};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server =
        CustomServer::<Example>::open(Path::new("server-data.pliantdb"), Configuration::default())
            .await?;
    server
        .set_custom_api_dispatcher(ApiDispatcher {
            server: server.clone(),
        })
        .await;
    server.register_schema::<()>().await?;
    let _ = server.create_database::<()>("counter").await;
    // Create a certificate if it doesn't exist.
    server.listen_for_websockets_on("127.0.0.1:8081").await?;

    Ok(())
}

#[derive(Debug)]
enum Example {}
impl Backend for Example {
    type CustomApi = ExampleApi;
    type CustomApiDispatcher = ApiDispatcher;
}

#[derive(Debug, actionable::Dispatcher)]
#[dispatcher(input = Request)]
struct ApiDispatcher {
    server: CustomServer<Example>,
}

impl RequestDispatcher for ApiDispatcher {
    type Output = Response;

    type Error = anyhow::Error;
}

#[actionable::async_trait]
impl IncrementCounterHandler for ApiDispatcher {
    async fn handle(&self, _permissions: &actionable::Permissions) -> anyhow::Result<Response> {
        let db = self.server.database::<()>("counter").await?;

        // TODO implement increment
        let mut current_value: u64 = db.get_key("current-count").await?.unwrap_or_default();
        current_value += 1;
        db.set_key("current-count", &current_value).await?;

        db.publish("counter-changed", &current_value).await?;
        Ok(Response::CounterIncremented(current_value))
    }
}
