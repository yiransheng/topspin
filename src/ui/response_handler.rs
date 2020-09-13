use druid::widget::{Controller, Widget};
use druid::{Env, Event, EventCtx};

use super::app_data::AppData;
use crate::constants::RUN_RESPONSES;

pub struct ResponseHandler;

impl ResponseHandler {
    pub fn new() -> Self {
        ResponseHandler
    }
}

impl<W> Controller<AppData, W> for ResponseHandler
where
    W: Widget<AppData>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppData,
        env: &Env,
    ) {
        let run_response = match event {
            Event::Command(cmd) if cmd.is(RUN_RESPONSES) => cmd.get_unchecked(RUN_RESPONSES),
            _ => return child.event(ctx, event, data, env),
        };
        data.handle_run_respone(run_response);
        ctx.request_paint();
    }
}
