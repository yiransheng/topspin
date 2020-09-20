use druid::lens::{Id, LensExt};
use druid::widget::{Align, Button, CrossAxisAlignment, Flex, FlexParams, Label, TextBox};
use druid::{self, im, Lens, Target, Widget, WidgetExt};

use super::app_data::{Entry, EntryData};
use crate::constants::SAVE_TO_FILE;

pub(super) fn new_entry() -> impl Widget<(EntryData, im::Vector<Entry>)> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Align::centered(
            Label::new("New Command").with_text_size(20.0),
        ))
        .with_spacer(8.0)
        .with_child(Label::new("Alias"))
        .with_child(
            TextBox::new()
                .expand_width()
                .lens(EntryData::alias)
                .lens(druid::lens!((EntryData, _), 0)),
        )
        .with_spacer(8.0)
        .with_child(Label::new("Command"))
        .with_child(
            TextBox::new()
                .expand_width()
                .lens(EntryData::command)
                .lens(druid::lens!((EntryData, _), 0)),
        )
        .with_spacer(8.0)
        .with_child(Label::new("Arguments"))
        .with_child(
            TextBox::new()
                .expand_width()
                .lens(EntryData::args)
                .lens(druid::lens!((EntryData, _), 0)),
        )
        .with_spacer(8.0)
        .with_child(Label::new("Working Directory"))
        .with_child(
            TextBox::new()
                .expand_width()
                .lens(opt_string_lens())
                .lens(EntryData::working_dir)
                .lens(druid::lens!((EntryData, _), 0)),
        )
        .with_spacer(16.0)
        .with_flex_child(
            Button::new("Done")
                .on_click(|ctx, data: &mut (EntryData, im::Vector<Entry>), _env| {
                    let entry_data = std::mem::replace(&mut data.0, EntryData::default());
                    data.1.push_back(Entry::new(entry_data));
                    ctx.submit_command(SAVE_TO_FILE, Some(Target::Global));
                })
                .fix_size(72.0, 32.0),
            FlexParams::new(1.0, CrossAxisAlignment::End),
        )
        .padding((4.0, 8.0))
}

fn opt_string_lens() -> impl Lens<Option<String>, String> {
    Id.map(
        |s: &Option<String>| s.clone().unwrap_or_else(String::new),
        |d: &mut Option<String>, s: String| {
            if s.trim().is_empty() {
                *d = None;
            } else {
                *d = Some(s);
            }
        },
    )
}
