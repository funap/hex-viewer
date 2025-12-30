use crate::core::command::Command;

/// Manages the history of commands for undo/redo functionality.
#[derive(Default)]
pub struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
}

impl History {
    /// Creates a new empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new command onto the undo stack and clears the redo stack.
    /// This should be called when a new command is executed.
    pub fn push(&mut self, command: Box<dyn Command>) {
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    /// Pops the last command from the undo stack.
    pub fn pop_undo(&mut self) -> Option<Box<dyn Command>> {
        self.undo_stack.pop()
    }

    /// Pushes a command onto the redo stack.
    pub fn push_redo(&mut self, command: Box<dyn Command>) {
        self.redo_stack.push(command);
    }

    /// Pops the last command from the redo stack.
    pub fn pop_redo(&mut self) -> Option<Box<dyn Command>> {
        self.redo_stack.pop()
    }

    /// Pushes a command back onto the undo stack (used during redo).
    pub fn push_undo(&mut self, command: Box<dyn Command>) {
        self.undo_stack.push(command);
    }

    /// Clears the history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}