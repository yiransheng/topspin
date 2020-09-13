use druid::lens::{self, LensExt};
use druid::widget::{Align, CrossAxisAlignment, Flex, Label, List, Scroll, TextBox, ViewSwitcher};
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
    Scroll::new(List::new(entry))
        .vertical()
        .lens(AppData::entries_lens())
}
