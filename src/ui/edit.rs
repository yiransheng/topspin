use druid::lens::{Id, LensExt};
use druid::widget::{Align, Button, CrossAxisAlignment, Flex, FlexParams, Label, TextBox};
use druid::{self, Lens, Target, Widget, WidgetExt};

use super::app_data::{EntryData};
use crate::constants::SAVE_TO_FILE;

pub(super) fn edit_entry() -> impl Widget<EntryData> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(Align::centered(
            Label::new("New Command").with_text_size(20.0),
        ))
        .with_spacer(8.0)
        .with_child(Label::new("Alias"))
        .with_child(TextBox::new().expand_width().lens(EntryData::alias))
        .with_spacer(8.0)
        .with_child(Label::new("Command"))
        .with_child(TextBox::new().expand_width().lens(EntryData::command))
        .with_spacer(8.0)
        .with_child(Label::new("Arguments"))
        .with_child(TextBox::new().expand_width().lens(EntryData::args))
        .with_spacer(8.0)
        .with_child(Label::new("Working Directory"))
        .with_child(
            TextBox::new()
                .expand_width()
                .lens(opt_string_lens())
                .lens(EntryData::working_dir),
        )
        .with_spacer(16.0)
        .with_flex_child(
            Button::new("Done")
                .on_click(|ctx, _data: &mut EntryData, _env| {
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
