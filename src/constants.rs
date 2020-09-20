use druid::Selector;

use crate::model::RunResponse;

pub const RUN_RESPONSES: Selector<RunResponse> = Selector::new("channel.run_response");
pub const SAVE_TO_FILE: Selector<()> = Selector::new("channel.save_to_file");

pub const STDOUT_TAG: u8 = 1;
pub const STDERR_TAG: u8 = 2;
