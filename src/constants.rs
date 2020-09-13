use druid::Selector;

use crate::model::RunResponse;

pub const RUN_RESPONSES: Selector<RunResponse> = Selector::new("channel.run_response");
