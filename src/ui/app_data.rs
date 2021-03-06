use std::iter::Iterator;

use druid::lens::{self, LensExt};
use druid::{im, Data, Lens};
use tokio::sync::mpsc;

use crate::model::{ProgramId, ProgramIdGen, RunCommand, RunRequest, RunResponse};
use crate::persist::{dump_entries, CommandEntry, Commands};

#[derive(Clone, Data, Lens)]
pub struct AppData {
    __id_counter: u32,
    pub entries: im::Vector<Entry>,
    pub edit_entry: EditState<EntryData>,

    #[data(ignore)]
    pub req_chan: mpsc::Sender<RunRequest>,
}

#[derive(Clone, Data, Eq, PartialEq)]
pub enum EditState<T> {
    New(T),
    Edit(usize, T),
    None,
}

impl<T> EditState<T> {
    #[allow(dead_code)]
    pub fn map<U, F>(&self, f: F) -> EditState<U>
    where
        F: Fn(&T) -> U,
    {
        match *self {
            EditState::New(ref t) => EditState::New(f(t)),
            EditState::Edit(i, ref t) => EditState::Edit(i, f(t)),
            EditState::None => EditState::None,
        }
    }

    pub fn map_to<U>(&self, v: U) -> EditState<U> {
        match *self {
            EditState::New(_) => EditState::New(v),
            EditState::Edit(i, _) => EditState::Edit(i, v),
            EditState::None => EditState::None,
        }
    }

    pub fn take(&mut self) -> Self {
        std::mem::replace(self, EditState::None)
    }

    pub fn data() -> impl Lens<EditState<T>, T>
    where
        T: Default,
    {
        EditStateLens
    }
}

struct EditStateLens;

impl<T: Default> Lens<EditState<T>, T> for EditStateLens {
    fn with<V, F: FnOnce(&T) -> V>(&self, data: &EditState<T>, f: F) -> V {
        match *data {
            EditState::New(ref x) => f(x),
            EditState::Edit(_, ref x) => f(x),
            EditState::None => f(&T::default()),
        }
    }
    fn with_mut<V, F: FnOnce(&mut T) -> V>(&self, data: &mut EditState<T>, f: F) -> V {
        match *data {
            EditState::New(ref mut x) => f(x),
            EditState::Edit(_, ref mut x) => f(x),
            EditState::None => f(&mut T::default()),
        }
    }
}

impl ProgramIdGen for AppData {
    fn counter(&mut self) -> &mut u32 {
        &mut self.__id_counter
    }
}

impl AppData {
    pub fn new(req_chan: mpsc::Sender<RunRequest>) -> Self {
        Self {
            __id_counter: 0,
            req_chan,
            edit_entry: EditState::None,
            entries: im::vector![],
        }
    }

    pub fn from_commands(commands: Commands, req_chan: mpsc::Sender<RunRequest>) -> Self {
        let entries = commands
            .into_iter()
            .map(EntryData::from)
            .map(Entry::new)
            .collect();
        Self {
            __id_counter: 0,
            req_chan,
            edit_entry: EditState::None,
            entries,
        }
    }

    pub fn done_editing(&mut self) {
        match self.edit_entry.take() {
            EditState::None => {}
            EditState::New(entry_data) => {
                self.entries.push_back(Entry::new(entry_data));
            }
            EditState::Edit(index, entry_data) => {
                if let Some(old_entry) = self.entries.get_mut(index) {
                    old_entry.data = entry_data;
                }
            }
        }
    }

    pub fn persist(&self) {
        let _ = dump_entries(self.entries.iter().map(|e| e.data.clone().into()));
    }

    pub fn entries_lens() -> impl Lens<AppData, (AppData, im::Vector<Entry>)> {
        lens::Id.map(
            |d: &AppData| (d.clone(), d.entries.clone()),
            |d: &mut AppData, x: (AppData, _)| {
                *d = x.0;
                if x.1.len() == d.entries.len() {
                    d.entries = x.1;
                }
            },
        )
    }

    pub fn handle_run_respone(&mut self, run_response: &RunResponse) {
        if let Some(entry) = self.find_entry(run_response.program_id()) {
            entry.state = entry.state.next(run_response);
            if let RunResponse::IoError(_, ref io_error) = run_response {
                entry.last_run_error = Some(format!("{}", io_error));
            } else {
                entry.last_run_error = None;
            }
        }
    }

    fn find_entry(&mut self, id: ProgramId) -> Option<&mut Entry> {
        self.entries.iter_mut().find_map(|entry| match entry.state {
            RunState::Idle(Some(state_id)) if state_id == id => Some(entry),
            RunState::Busy(state_id) if state_id == id => Some(entry),
            RunState::Running(state_id, _) if state_id == id => Some(entry),
            _ => None,
        })
    }
}

#[derive(Clone, Data, Lens, Eq, PartialEq)]
pub struct Entry {
    pub(super) data: EntryData,
    pub(super) state: RunState,
    pub(super) last_run_error: Option<String>,
}

impl Entry {
    pub(super) fn new(data: EntryData) -> Self {
        Self {
            data,
            state: RunState::default(),
            last_run_error: None,
        }
    }
}

#[derive(Copy, Clone, Data, Eq, PartialEq)]
pub enum RunState {
    Idle(Option<ProgramId>),
    Busy(ProgramId),
    // (internal_id, PID)
    Running(ProgramId, u32),
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
                RunResponse::Started(started_id, pid) if id == started_id => {
                    RunState::Running(id, pid)
                }
                RunResponse::Exited(exit_id, _) if id == exit_id => RunState::Idle(Some(id)),
                _ => self,
            },
            RunState::Running(id, _) => match *res {
                RunResponse::Exited(exit_id, _) if id == exit_id => RunState::Idle(Some(id)),
                _ => self,
            },
        }
    }
}

#[derive(Clone, Default, Data, Lens, Eq, PartialEq)]
pub struct EntryData {
    pub(super) alias: String,
    pub(super) command: String,
    pub(super) args: String,
    pub(super) working_dir: Option<String>,
}

impl From<(String, CommandEntry)> for EntryData {
    fn from((alias, command_entry): (String, CommandEntry)) -> Self {
        let CommandEntry {
            command,
            args,
            working_dir,
        } = command_entry;
        EntryData {
            alias,
            command: command,
            args: args.unwrap_or_else(String::new),
            working_dir,
        }
    }
}

impl Into<(String, CommandEntry)> for EntryData {
    fn into(self) -> (String, CommandEntry) {
        (
            self.alias,
            CommandEntry {
                command: self.command,
                args: Some(self.args).filter(|s| !s.is_empty()),
                working_dir: self.working_dir,
            },
        )
    }
}

impl EntryData {
    pub(super) fn make_command(&self, id: ProgramId) -> Result<RunCommand, String> {
        let args =
            shell_words::split(&self.args).map_err(|_| "Invalid command arguments".to_string())?;
        Ok(RunCommand {
            id,
            alias: self.alias.clone(),
            name: self.command.clone(),
            args,
            working_dir: self.working_dir.clone(),
        })
    }
}
