use druid::lens::{self, LensExt};
use druid::widget::{
    Align, Button, CrossAxisAlignment, Flex, Label, List, Scroll, TextBox, ViewSwitcher,
};
use druid::{
    im, AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};

pub mod app_data;

mod create;
mod entry;
mod response_handler;

use self::app_data::{AppData, Entry, EntryData};
use self::create::new_entry;
use self::entry::entry;
use self::response_handler::ResponseHandler;

pub fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();
    let child = ViewSwitcher::new(
        |app_data: &AppData, _| app_data.new_entry.as_ref().map(|_| ()),
        |selector, _data, _env| match selector {
            Some(_entry) => Box::new(new_entry().lens(lens::Id.map(
                |d: &AppData| (d.new_entry.clone().unwrap(), d.entries.clone()),
                |d: &mut AppData, x: (EntryData, im::Vector<Entry>)| {
                    if let Some(ref mut data) = d.new_entry {
                        *data = x.0;
                    }
                    // New entry added
                    if d.entries.len() < x.1.len() {
                        d.new_entry = None;
                    }
                    d.entries = x.1;
                },
            ))),
            None => Box::new(list_view()),
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
                    app_data.new_entry = Some(EntryData::default())
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
