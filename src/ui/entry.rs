use druid::widget::{
    Align, Button, Container, CrossAxisAlignment, Flex, Label, MainAxisAlignment, TextBox,
    ViewSwitcher,
};
use druid::{
    self, AppLauncher, Color, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget,
    WidgetExt, WindowDesc,
};

use super::app_data::{AppData, Entry, EntryData, RunState};
use crate::model::{ProgramIdGen, RunRequest};

pub(super) fn entry() -> impl Widget<(AppData, Entry)> {
    Container::new(
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_flex_child(
                entry_data()
                    .lens(Entry::data)
                    .lens(druid::lens!((AppData, Entry), 1)),
                8.0,
            )
            .with_flex_child(actions(), 2.0),
    )
    .padding(4.0)
    .border(Color::WHITE, 1.0)
}

fn entry_data() -> impl Widget<EntryData> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Label::new(|data: &EntryData, _env: &Env| {
            data.alias.clone()
        }))
        .with_spacer(4.0)
        .with_child(Label::new(|data: &EntryData, _env: &Env| {
            format!("{} {}", &data.command, &data.args)
        }))
}

const IDLE: u32 = 0;
const BUSY: u32 = 1;
const RUNNING: u32 = 2;

fn actions() -> impl Widget<(AppData, Entry)> {
    ViewSwitcher::new(
        |(_, entry): &(_, Entry), _env| match entry.state {
            RunState::Idle(_) => IDLE,
            RunState::Busy(_) => BUSY,
            RunState::Running(_) => RUNNING,
        },
        |selector, _data, _env| match *selector {
            IDLE => Box::new(start_button()),
            RUNNING => Box::new(kill_button()),
            _ => Box::new(Label::new("waiting...")),
        },
    )
}

fn start_button() -> impl Widget<(AppData, Entry)> {
    Button::new("Start").on_click(|_ctx, (app_data, entry): &mut (AppData, Entry), _env| {
        let id = match entry.state {
            RunState::Idle(ref mut program_id) => program_id.unwrap_or_else(|| app_data.next_id()),
            _ => return,
        };
        let run_request = RunRequest::Run(entry.data.make_command(id));

        let mut tx = app_data.req_chan.clone();
        tokio::spawn(async move {
            tx.send(run_request).await;
        });

        entry.state = RunState::Busy(id);
    })
}

fn kill_button() -> impl Widget<(AppData, Entry)> {
    Button::new("Kill").on_click(|_ctx, (app_data, entry): &mut (AppData, Entry), _env| {
        let id = match entry.state {
            RunState::Running(id) => id,
            _ => return,
        };
        let kill_request = RunRequest::Kill(id);
        let mut tx = app_data.req_chan.clone();
        tokio::spawn(async move {
            tx.send(kill_request).await;
        });

        entry.state = RunState::Busy(id);
    })
}
