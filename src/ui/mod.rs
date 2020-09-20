use druid::lens::LensExt;
use druid::widget::{Button, CrossAxisAlignment, Flex, List, Scroll, ViewSwitcher};
use druid::{AppDelegate, Command, DelegateCtx, Env, Target, Widget, WidgetExt};

pub mod app_data;

mod edit;
mod entry;
mod response_handler;

use self::app_data::{AppData, EditState, EntryData};
use self::edit::edit_entry;
use self::entry::entry;
use self::response_handler::ResponseHandler;
use crate::constants::SAVE_TO_FILE;

pub struct Delegate {}

impl Delegate {
    pub fn new() -> Self {
        Delegate {}
    }
}

impl AppDelegate<AppData> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppData,
        _env: &Env,
    ) -> bool {
        if let Some(_) = cmd.get(SAVE_TO_FILE) {
            data.done_editing();
            let _ = data.persist();
        }
        true
    }
}

pub fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();
    let child = ViewSwitcher::new(
        |app_data: &AppData, _| app_data.edit_entry.map_to(()),
        |selector, _data, _env| match selector {
            EditState::New(_) | EditState::Edit(..) => Box::new(
                edit_entry()
                    .lens(EditState::data())
                    .lens(AppData::edit_entry),
            ),
            EditState::None => Box::new(list_view()),
        },
    );

    root.add_flex_child(child, 1.0);
    root.controller(ResponseHandler::new())
}

fn list_view() -> impl Widget<AppData> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Button::new("New Command")
                .on_click(|_ctx, app_data: &mut AppData, _env| {
                    app_data.edit_entry = EditState::New(EntryData::default())
                })
                .fix_height(32.0)
                .padding((3.0, 0.0))
                .expand_width(),
        )
        .with_child(
            Scroll::new(List::new(entry))
                .vertical()
                .lens(AppData::entries_lens()),
        )
}
