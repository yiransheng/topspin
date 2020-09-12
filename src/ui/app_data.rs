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

#[derive(Clone, Data, Lens)]
pub struct Entry {
    entry: EntryData,
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
    alias: String,
    command: String,
    args: String,
}
