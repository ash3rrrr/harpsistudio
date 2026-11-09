//! Bounded snapshot undo/redo history.
//!
//! Harpsistudio already keeps a mutable `Song` on the UI thread and publishes
//! immutable snapshots to the audio engine. Reusing that model for history is
//! the cheapest coherent design: each completed gesture adds one `Song` clone
//! to a bounded ring, and undo/redo simply republishes a previous snapshot.
//! No history state ever lives on the audio thread.

use std::collections::VecDeque;

/// Maximum number of retained project snapshots, including the current one.
pub const DEFAULT_UNDO_LIMIT: usize = 64;

/// A bounded, linear undo/redo stack over whole project snapshots.
#[derive(Clone, Debug)]
pub struct UndoHistory<T> {
    states: VecDeque<T>,
    /// Index of the snapshot currently represented by the live project.
    cursor: usize,
    limit: usize,
}

impl<T: Clone + PartialEq> UndoHistory<T> {
    /// Start a history with the current project state as its baseline.
    pub fn new(initial: T, limit: usize) -> Self {
        let limit = limit.max(1);
        let mut states = VecDeque::with_capacity(limit.min(8));
        states.push_back(initial);
        Self {
            states,
            cursor: 0,
            limit,
        }
    }

    /// Commit one completed gesture, dropping any redo branch. Returns false
    /// when the state is identical to the cursor (no-op edits never grow the
    /// history).
    pub fn record(&mut self, state: T) -> bool {
        if self.current() == Some(&state) {
            return false;
        }
        while self.states.len() > self.cursor + 1 {
            self.states.pop_back();
        }
        self.states.push_back(state);
        self.cursor = self.states.len() - 1;
        while self.states.len() > self.limit {
            self.states.pop_front();
            self.cursor = self.cursor.saturating_sub(1);
        }
        true
    }

    /// Move one step back and return that snapshot.
    pub fn undo(&mut self) -> Option<T> {
        if !self.can_undo() {
            return None;
        }
        self.cursor -= 1;
        self.states.get(self.cursor).cloned()
    }

    /// Move one step forward and return that snapshot.
    pub fn redo(&mut self) -> Option<T> {
        if !self.can_redo() {
            return None;
        }
        self.cursor += 1;
        self.states.get(self.cursor).cloned()
    }

    pub fn current(&self) -> Option<&T> {
        self.states.get(self.cursor)
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.states.len()
    }

    /// Number of retained snapshots (including the current one).
    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(states: &[i32], limit: usize) -> UndoHistory<i32> {
        let mut history = UndoHistory::new(states[0], limit);
        for state in &states[1..] {
            history.record(*state);
        }
        history
    }

    #[test]
    fn undo_and_redo_follow_edit_order() {
        let mut h = history(&[0, 1, 2], 8);
        assert_eq!(h.undo(), Some(1));
        assert_eq!(h.undo(), Some(0));
        assert_eq!(h.undo(), None);
        assert_eq!(h.redo(), Some(1));
        assert_eq!(h.redo(), Some(2));
        assert_eq!(h.redo(), None);
    }

    #[test]
    fn a_new_edit_invalidates_the_redo_branch() {
        let mut h = history(&[0, 1, 2], 8);
        assert_eq!(h.undo(), Some(1));
        assert!(h.record(9));
        assert!(!h.can_redo());
        assert_eq!(h.redo(), None);
        assert_eq!(h.current(), Some(&9));
    }

    #[test]
    fn history_depth_is_bounded_and_evicts_oldest_states() {
        let mut h = history(&[0, 1, 2, 3, 4], 3);
        assert_eq!(h.len(), 3);
        assert_eq!(h.undo(), Some(3));
        assert_eq!(h.undo(), Some(2));
        assert_eq!(h.undo(), None);
    }

    #[test]
    fn a_batched_gesture_records_exactly_one_state() {
        let mut h = UndoHistory::new(0, 8);
        let mut gesture = 0;
        for _ in 0..200 {
            gesture += 1;
        }
        assert!(h.record(gesture));
        assert_eq!(h.len(), 2);
        assert_eq!(h.undo(), Some(0));
        assert_eq!(h.redo(), Some(200));
    }

    #[test]
    fn identical_states_are_ignored() {
        let mut h = UndoHistory::new(7, 8);
        assert!(!h.record(7));
        assert_eq!(h.len(), 1);
        assert!(!h.can_undo());
    }
}
