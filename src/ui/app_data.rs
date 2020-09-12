use druid::lens::{self, LensExt};
use druid::{
    im, AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};

use crate::model::{ProgramId, ProgramIdGen};

#[derive(Clone, Data, Lens)]
pub struct AppData {
    __id_counter: u32,
    entries: im::Vector<Entry>,
}

impl ProgramIdGen for AppData {
    fn counter(&mut self) -> &mut u32 {
        &mut self.__id_counter
    }
}

impl AppData {
    pub fn entries_lens() -> impl Lens<AppData, (AppData, im::Vector<Entry>)> {
        lens::Id.map(
            |d: &AppData| (d.clone(), d.entries.clone()),
            |d: &mut AppData, x: (AppData, _)| {
                *d = x.0;
            },
        )
    }
}

#[derive(Clone, Data, Lens)]
pub struct Entry {
    data: EntryData,
    state: RunState,
}

#[derive(Copy, Clone, Data)]
pub enum RunState {
    Idle(Option<ProgramId>),
    Busy(ProgramId),
    Running(ProgramId),
}

impl Default for RunState {
    fn default() -> Self {
        RunState::Idle(None)
    }
}

#[derive(Clone, Data, Lens)]
pub struct EntryData {
    pub(super) alias: String,
    pub(super) command: String,
    pub(super) args: String,
}

pub fn new_app_data() -> AppData {
    AppData {
        __id_counter: 0,
        entries: im::vector![
            Entry {
                state: RunState::default(),
                data: EntryData {
                    alias: "cat".into(),
                    command: "cat".into(),
                    args: String::new(),
                }
            },
            Entry {
                state: RunState::default(),
                data: EntryData {
                    alias: "netcat".into(),
                    command: "nc".into(),
                    args: "-l 7000".into(),
                }
            }
        ],
    }
}
