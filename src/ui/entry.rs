use druid::widget::{
    Align, Container, CrossAxisAlignment, Flex, Label, MainAxisAlignment, TextBox,
};
use druid::{
    self, AppLauncher, Color, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget,
    WidgetExt, WindowDesc,
};

use super::app_data::{AppData, Entry, EntryData};

pub(super) fn entry() -> impl Widget<(AppData, Entry)> {
    Container::new(
        Flex::row()
            .main_axis_alignment(MainAxisAlignment::Start)
            .must_fill_main_axis(true)
            .with_flex_child(
                entry_data()
                    .lens(Entry::data)
                    .lens(druid::lens!((AppData, Entry), 1)),
                1.0,
            ),
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
