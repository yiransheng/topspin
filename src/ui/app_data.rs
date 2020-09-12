use std::iter::Iterator;

use druid::lens::{self, LensExt};
use druid::{im, Data, Lens};
use tokio::sync::mpsc;

use crate::model::{ProgramId, ProgramIdGen, RunCommand, RunRequest, RunResponse};

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

    pub fn handle_run_respone(&mut self, run_response: &RunResponse) {
        if let Some(entry) = self.find_entry(run_response.program_id()) {
            entry.state = entry.state.next(run_response);
        }
    }

    fn find_entry(&mut self, id: ProgramId) -> Option<&mut Entry> {
        self.entries.iter_mut().find_map(|entry| match entry.state {
            RunState::Idle(Some(state_id)) if state_id == id => Some(entry),
            RunState::Busy(state_id) if state_id == id => Some(entry),
            RunState::Running(state_id) if state_id == id => Some(entry),
            _ => None,
        })
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

impl RunState {
    pub(super) fn next(self, res: &RunResponse) -> Self {
        if let RunResponse::IoError(_, _) = *res {
            return RunState::Idle(None);
        }
        match self {
            RunState::Idle(_) => self,
            RunState::Busy(id) => match *res {
                RunResponse::Started(started_id, _) if id == started_id => RunState::Running(id),
                RunResponse::Exited(exit_id, _) if id == exit_id => RunState::Idle(Some(id)),
                _ => self,
            },
            RunState::Running(id) => match *res {
                RunResponse::Exited(exit_id, _) if id == exit_id => RunState::Idle(Some(id)),
                _ => self,
            },
        }
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
