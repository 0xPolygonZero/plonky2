#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use log::{log, Level};

/// The hierarchy of contexts, and the gate count contributed by each one. Useful for debugging.
#[derive(Debug)]
pub(crate) struct ContextTree {
    /// The name of this scope.
    name: String,
    /// The level at which to log this scope and its children.
    level: log::Level,
    /// The gate count when this scope was created.
    enter_gate_count: usize,
    /// The gate count when this scope was destroyed, or None if it has not yet been destroyed.
    exit_gate_count: Option<usize>,
    /// Any child contexts.
    children: Vec<ContextTree>,
}

impl ContextTree {
    pub fn new() -> Self {
        Self {
            name: "root".to_string(),
            level: Level::Debug,
            enter_gate_count: 0,
            exit_gate_count: None,
            children: vec![],
        }
    }

    /// Whether this context is still in scope.
    const fn is_open(&self) -> bool {
        self.exit_gate_count.is_none()
    }

    /// A description of the stack of currently-open scopes.
    pub fn open_stack(&self) -> String {
        let mut stack = Vec::new();
        self.open_stack_helper(&mut stack);
        stack.join(" > ")
    }

    fn open_stack_helper(&self, stack: &mut Vec<String>) {
        if self.is_open() {
            stack.push(self.name.clone());
            if let Some(last_child) = self.children.last() {
                last_child.open_stack_helper(stack);
            }
        }
    }

    pub fn push(&mut self, ctx: &str, mut level: log::Level, current_gate_count: usize) {
        assert!(self.is_open());

        // We don't want a scope's log level to be stronger than that of its parent.
        level = level.max(self.level);

        if let Some(last_child) = self.children.last_mut() {
            if last_child.is_open() {
                last_child.push(ctx, level, current_gate_count);
                return;
            }
        }

        self.children.push(ContextTree {
            name: ctx.to_string(),
            level,
            enter_gate_count: current_gate_count,
            exit_gate_count: None,
            children: vec![],
        })
    }

    /// Close the deepest open context from this tree.
    pub fn pop(&mut self, current_gate_count: usize) {
        assert!(self.is_open());

        if let Some(last_child) = self.children.last_mut() {
            if last_child.is_open() {
                last_child.pop(current_gate_count);
                return;
            }
        }

        self.exit_gate_count = Some(current_gate_count);
    }

    fn gate_count_delta(&self, current_gate_count: usize) -> usize {
        self.exit_gate_count.unwrap_or(current_gate_count) - self.enter_gate_count
    }

    /// Filter out children with a low gate count.
    pub fn filter(&self, current_gate_count: usize, min_delta: usize) -> Self {
        Self {
            name: self.name.clone(),
            level: self.level,
            enter_gate_count: self.enter_gate_count,
            exit_gate_count: self.exit_gate_count,
            children: self
                .children
                .iter()
                .filter(|c| c.gate_count_delta(current_gate_count) >= min_delta)
                .map(|c| c.filter(current_gate_count, min_delta))
                .collect(),
        }
    }

    pub fn print(&self, current_gate_count: usize) {
        self.print_helper(current_gate_count, 0);
    }

    fn print_helper(&self, current_gate_count: usize, depth: usize) {
        let prefix = "| ".repeat(depth);
        log!(
            self.level,
            "{}{} gates to {}",
            prefix,
            self.gate_count_delta(current_gate_count),
            self.name
        );
        for child in &self.children {
            child.print_helper(current_gate_count, depth + 1);
        }
    }
}

/// Creates a named scope; useful for debugging.
#[macro_export]
macro_rules! with_context {
    ($builder:expr, $level:expr, $ctx:expr, $exp:expr) => {{
        $builder.push_context($level, $ctx);
        let res = $exp;
        $builder.pop_context();
        res
    }};
    // If no context is specified, default to Debug.
    ($builder:expr, $ctx:expr, $exp:expr) => {{
        $builder.push_context(log::Level::Debug, $ctx);
        let res = $exp;
        $builder.pop_context();
        res
    }};
}
