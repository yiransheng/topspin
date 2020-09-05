use std::mem;
use std::time::Instant;

#[derive(Debug, Copy, Clone, Ord, Eq, PartialOrd, PartialEq, Hash)]
pub struct ProgramId(u32);

#[cfg(test)]
pub fn program_id(id: u32) -> ProgramId {
    ProgramId(id)
}

#[derive(Debug, Clone)]
pub struct RunCommand {
    pub id: ProgramId,
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum RunRequest {
    Run(RunCommand),
    Kill(ProgramId),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RunState {
    NotRunning,
    // Unresponsive,
    Spawning,
    Dying,
    RunningSince(Instant),
}

impl Default for RunState {
    fn default() -> Self {
        RunState::NotRunning
    }
}

pub struct ProgramMap<V> {
    elements: Vec<Option<V>>,
}

impl<V> ProgramMap<V> {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
        }
    }

    pub fn get(&self, key: ProgramId) -> Option<&V> {
        let index = key.0 as usize;
        self.elements.get(index).map(Option::as_ref).flatten()
    }

    pub fn get_mut(&mut self, key: ProgramId) -> Option<&mut V> {
        let index = key.0 as usize;
        self.elements.get_mut(index).map(Option::as_mut).flatten()
    }

    pub fn insert(&mut self, key: ProgramId, value: V) -> Option<V> {
        let index = key.0 as usize;
        if index < self.elements.len() {
            if let Some(old_value) = self.get_mut(key) {
                Some(mem::replace(old_value, value))
            } else {
                self.elements[index] = Some(value);
                None
            }
        } else {
            while self.elements.len() < index {
                self.elements.push(None);
            }
            self.elements.push(Some(value));
            None
        }
    }

    pub fn remove(&mut self, key: ProgramId) -> Option<V> {
        let index = key.0 as usize;
        if let Some(old_value) = self.elements.get_mut(index) {
            mem::replace(old_value, None)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_map() {
        let mut map: ProgramMap<String> = ProgramMap::new();
        let key = ProgramId(4);
        map.insert(key, "A".to_string());
        let prev = map.insert(key, "B".to_string()).unwrap();
        assert_eq!(prev, "A");
        assert_eq!(map.get(key).unwrap(), "B");
    }
}
