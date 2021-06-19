use pliantdb::core::custom_api::CustomApi;
use serde::{Deserialize, Serialize};

pub const DATABASE_NAME: &str = "counter";
pub const COUNTER_CHANGED_TOPIC: &str = "counter-changed";

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "actionable-traits", derive(actionable::Actionable))]
pub enum Request {
    #[cfg_attr(feature = "actionable-traits", actionable(protection = "none"))]
    IncrementCounter,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Response {
    CounterIncremented(u64),
}

#[derive(Debug)]
pub enum ExampleApi {}

impl CustomApi for ExampleApi {
    type Request = Request;
    type Response = Response;
}
