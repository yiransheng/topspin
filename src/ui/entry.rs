use druid::widget::{
    Align, Button, Container, CrossAxisAlignment, Flex, FlexParams, Label, MainAxisAlignment,
    Padding, TextBox, ViewSwitcher,
};
use druid::{
    self, AppLauncher, Color, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget,
    WidgetExt, WindowDesc,
};

use super::app_data::{AppData, Entry, RunState};
use crate::model::{ProgramIdGen, RunRequest};

pub(super) fn entry() -> impl Widget<(AppData, Entry)> {
    Padding::new(
        4.0,
        Container::new(
            Flex::row()
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .with_flex_child(entry_data().lens(druid::lens!((AppData, Entry), 1)), 2.0)
                .with_flex_child(actions(), 1.0),
        )
        .padding((8.0, 4.0))
        .border(Color::grey8(222), 1.0),
    )
}

fn entry_data() -> impl Widget<Entry> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::new(|entry: &Entry, _env: &Env| {
                let state = entry.state;
                match state {
                    RunState::Running(_, pid) => format!("{} (PID: {})", &entry.data.alias, pid),
                    _ => entry.data.alias.clone(),
                }
            })
            .with_text_size(20.0)
            .expand_width(),
        )
        .with_spacer(4.0)
        .with_child(
            Label::new(|entry: &Entry, _env: &Env| {
                format!("{} {}", &entry.data.command, &entry.data.args)
            })
            .with_text_size(14.0),
        )
}

fn actions() -> impl Widget<(AppData, Entry)> {
    ViewSwitcher::new(
        |(_, entry): &(_, Entry), _env| entry.state,
        |selector, _data, _env| match *selector {
            RunState::Idle(..) => Box::new(start_button()),
            RunState::Running(..) => Box::new(kill_button()),
            _ => Box::new(Label::new("waiting...")),
        },
    )
    .align_right()
}

fn start_button() -> impl Widget<(AppData, Entry)> {
    let start = Button::new("Start")
        .on_click(|_ctx, (app_data, entry): &mut (AppData, Entry), _env| {
            let id = match entry.state {
                RunState::Idle(ref mut program_id) => {
                    program_id.unwrap_or_else(|| app_data.next_id())
                }
                _ => return,
            };
            let run_request = RunRequest::Run(entry.data.make_command(id));

            let mut tx = app_data.req_chan.clone();
            tokio::spawn(async move {
                tx.send(run_request).await;
            });

            entry.state = RunState::Busy(id);
        })
        .fix_size(72.0, 32.0);

    let delete = Button::new("Delete")
        .on_click(|_ctx, (app_data, entry): &mut (AppData, Entry), _env| {
            if let RunState::Idle(_) = entry.state {
                app_data.entries.retain(|e| e != entry);
            }
        })
        .fix_size(72.0, 32.0);

    Flex::row()
        .with_child(delete)
        .with_spacer(12.0)
        .with_child(start)
}

fn kill_button() -> impl Widget<(AppData, Entry)> {
    Button::new("Kill")
        .on_click(|_ctx, (app_data, entry): &mut (AppData, Entry), _env| {
            let id = match entry.state {
                RunState::Running(id, _) => id,
                _ => return,
            };
            let kill_request = RunRequest::Kill(id);
            let mut tx = app_data.req_chan.clone();
            tokio::spawn(async move {
                tx.send(kill_request).await;
            });

            entry.state = RunState::Busy(id);
        })
        .fix_size(72.0, 32.0)
}
