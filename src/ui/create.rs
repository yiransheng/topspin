use druid::widget::{
    Align, Button, Container, CrossAxisAlignment, Flex, FlexParams, Label, MainAxisAlignment,
    Padding, TextBox, ViewSwitcher,
};
use druid::{
    self, im, AppLauncher, Color, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget,
    WidgetExt, WindowDesc,
};

use super::app_data::{AppData, Entry, EntryData, RunState};

pub(super) fn new_entry() -> impl Widget<(EntryData, im::Vector<Entry>)> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Align::centered(
            Label::new("New Command").with_text_size(20.0),
        ))
        .with_child(Label::new("Alias"))
        .with_child(
            TextBox::new()
                .lens(EntryData::alias)
                .lens(druid::lens!((EntryData, _), 0)),
        )
}
