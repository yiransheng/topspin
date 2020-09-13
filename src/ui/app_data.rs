use std::iter::Iterator;

use druid::lens::{self, LensExt};
use druid::{im, Data, Lens};
use tokio::sync::mpsc;

use crate::model::{ProgramId, ProgramIdGen, RunCommand, RunRequest, RunResponse};
use crate::persist::{CommandEntry, Commands};

#[derive(Clone, Data, Lens)]
pub struct AppData {
    __id_counter: u32,
    pub entries: im::Vector<Entry>,
    pub new_entry: Option<EntryData>,

    #[data(ignore)]
    pub req_chan: mpsc::Sender<RunRequest>,
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
            new_entry: None,
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
            new_entry: None,
            entries,
        }
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
}

impl Entry {
    pub(super) fn new(data: EntryData) -> Self {
        Self {
            data,
            state: RunState::default(),
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ValidationError {
    MissingAlias,
    MissingCommand,
    WhileSpaceInCommand,
    BadArgs,
}

impl Into<&'static str> for ValidationError {
    fn into(self) -> &'static str {
        match self {
            ValidationError::MissingAlias => "Alias cannot be emtpy",
            ValidationError::MissingCommand => "Command cannot be emtpy",
            ValidationError::WhileSpaceInCommand => "Command cannot contain whitespaces",
            ValidationError::BadArgs => "Error parsing shell arguments",
        }
    }
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

impl EntryData {
    pub(super) fn make_command(&self, id: ProgramId) -> RunCommand {
        let args = shell_words::split(&self.args).expect("Bad args");
        RunCommand {
            id,
            name: self.command.clone(),
            args,
            working_dir: self.working_dir.clone(),
        }
    }

    pub(super) fn validated(&self) -> Result<Self, Vec<ValidationError>> {
        let mut errors = vec![];
        if self.alias.trim().is_empty() {
            errors.push(ValidationError::MissingAlias);
        }
        if self.command.trim().is_empty() {
            errors.push(ValidationError::MissingCommand);
        }
        if self.command.chars().any(char::is_whitespace) {
            errors.push(ValidationError::WhileSpaceInCommand);
        }
        if shell_words::split(&self.args).is_err() {
            errors.push(ValidationError::BadArgs);
        }
        if errors.is_empty() {
            Ok(Self {
                alias: self.alias.trim().to_string(),
                command: self.command.trim().to_string(),
                args: self.args.clone(),
                working_dir: self.working_dir.clone(),
            })
        } else {
            Err(errors)
        }
    }
}
