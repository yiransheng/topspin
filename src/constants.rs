use druid::Selector;

use crate::model::RunResponse;

pub const RUN_RESPONSES: Selector<RunResponse> = Selector::new("channel.run_response");

pub const STDOUT_TAG: [u8; 1] = [1];
pub const STDERR_TAG: [u8; 1] = [2];
