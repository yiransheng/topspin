use druid::widget::{Align, CrossAxisAlignment, Flex, Label, List, Scroll, TextBox};
use druid::{
    AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};

pub mod app_data;
mod entry;

use self::app_data::AppData;
use self::entry::entry;

pub fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();
    let list = List::new(entry);
    let list = Scroll::new(list).vertical().lens(AppData::entries_lens());

    root.add_flex_child(list, 1.0);
    root
}
