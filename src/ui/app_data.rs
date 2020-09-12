use druid::lens::{self, LensExt};
use druid::{
    im, AppLauncher, Data, Env, ExtEventSink, Lens, LocalizedString, Selector, Widget, WidgetExt,
    WindowDesc,
};
use tokio::sync::mpsc;

use crate::model::{ProgramId, ProgramIdGen, RunCommand, RunRequest};

#[derive(Clone, Data, Lens)]
pub struct AppData {
    __id_counter: u32,
    entries: im::Vector<Entry>,

    #[data(ignore)]
    pub req_chan: mpsc::Sender<RunRequest>,
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
                d.entries = x.1;
            },
        )
    }
}

#[derive(Clone, Data, Lens)]
pub struct Entry {
    pub(super) data: EntryData,
    pub(super) state: RunState,
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

impl EntryData {
    pub(super) fn make_command(&self, id: ProgramId) -> RunCommand {
        let args = shell_words::split(&self.args).expect("Bad args");
        RunCommand {
            id,
            name: self.command.clone(),
            args,
        }
    }
}

pub fn new_app_data(req_chan: mpsc::Sender<RunRequest>) -> AppData {
    AppData {
        __id_counter: 0,
        req_chan,
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
