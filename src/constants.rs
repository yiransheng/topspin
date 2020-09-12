use druid::{Key, Selector};
use tokio::sync::mpsc;

use crate::model::{RunRequest, RunResponse};

pub const RUN_REQUESTS: Key<mpsc::Sender<RunRequest>> = Key::new("channel.run_requests");

pub const RUN_RESPONSES: Selector<RunResponse> = Selector::new("channel.run_response");
