pub struct Program {
    name: String,
    command: String,
}

impl Program {
    pub fn new(name: String, command: String) -> Self {
        Program { name, command }
    }
}
