// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Core queue functionality for event normalization.

use std::{hash::Hash, mem};

use linked_hash_map::LinkedHashMap;

use crate::event::Metadata;

use super::emitter::Emitter;

/// Normalization queue for incoming events.
///
/// We use [`LinkedHashMap`] everywhere throughout this module to ensure FIFO
/// queue for our events. This means by calling [`next()`] we reliably get the
/// currently outputting item's events. We're doing that until it yields an
/// event that corresponds to the item being finished, after which we remove the
/// current item, as all its events have been printed out, and we should do it
/// all over again with a [`next()`] item.
///
/// [`next()`]: Iterator::next()
#[derive(Clone, Debug)]
pub struct Queue<K: Eq + Hash, V> {
    /// Underlying FIFO queue of values.
    pub(super) fifo: LinkedHashMap<K, V>,

    /// Initial [`Metadata`] of this [`Queue`] creation.
    ///
    /// If this value is [`Some`], then `Started` [`Event`] hasn't been passed
    /// on to the inner [`Writer`] yet.
    ///
    /// [`Event`]: crate::Event
    /// [`Writer`]: crate::Writer
    pub(super) initial: Option<Metadata>,

    /// [`FinishedState`] of this [`Queue`].
    pub(super) state: FinishedState,
}

impl<K: Eq + Hash, V> Queue<K, V> {
    /// Creates a new normalization [`Queue`] with an initial metadata.
    pub fn new(initial: Metadata) -> Self {
        Self {
            fifo: LinkedHashMap::new(),
            initial: Some(initial),
            state: FinishedState::NotFinished,
        }
    }

    /// Marks this [`Queue`] as [`FinishedButNotEmitted`].
    ///
    /// [`FinishedButNotEmitted`]: FinishedState::FinishedButNotEmitted
    pub const fn finished(&mut self, meta: Metadata) {
        self.state = FinishedState::FinishedButNotEmitted(meta);
    }

    /// Checks whether this [`Queue`] transited to [`FinishedAndEmitted`] state.
    ///
    /// [`FinishedAndEmitted`]: FinishedState::FinishedAndEmitted
    pub const fn is_finished_and_emitted(&self) -> bool {
        matches!(self.state, FinishedState::FinishedAndEmitted)
    }

    /// Removes the given `key` from this [`Queue`].
    pub fn remove(&mut self, key: &K) {
        drop(self.fifo.remove(key));
    }
}


/// Finishing state of a [`Queue`].
#[derive(Clone, Copy, Debug)]
pub enum FinishedState {
    /// `Finished` event hasn't been encountered yet.
    NotFinished,

    /// `Finished` event has been encountered, but not passed to the inner
    /// [`Writer`] yet.
    ///
    /// This happens when output is busy due to outputting some other item.
    ///
    /// [`Writer`]: crate::Writer
    FinishedButNotEmitted(Metadata),

    /// `Finished` event has been encountered and passed to the inner
    /// [`Writer`].
    ///
    /// [`Writer`]: crate::Writer
    FinishedAndEmitted,
}

impl FinishedState {
    /// Returns [`Metadata`] of this [`FinishedState::FinishedButNotEmitted`],
    /// and makes it [`FinishedAndEmitted`].
    ///
    /// [`FinishedAndEmitted`]: FinishedState::FinishedAndEmitted
    pub const fn take_to_emit(&mut self) -> Option<Metadata> {
        let current = mem::replace(self, Self::FinishedAndEmitted);
        if let Self::FinishedButNotEmitted(meta) = current {
            Some(meta)
        } else {
            *self = current;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Metadata;

    #[test]
    fn test_queue_new() {
        let meta = Metadata::new(());
        let queue: Queue<String, i32> = Queue::new(meta);
        
        assert!(queue.initial.is_some());
        assert!(matches!(queue.state, FinishedState::NotFinished));
        assert_eq!(queue.fifo.len(), 0);
    }

    #[test]
    fn test_queue_finished() {
        let meta = Metadata::new(());
        let mut queue: Queue<String, i32> = Queue::new(meta);
        
        let finish_meta = Metadata::new(());
        queue.finished(finish_meta);
        
        assert!(matches!(queue.state, FinishedState::FinishedButNotEmitted(_)));
        assert!(!queue.is_finished_and_emitted());
    }

    #[test]
    fn test_queue_is_finished_and_emitted() {
        let meta = Metadata::new(());
        let mut queue: Queue<String, i32> = Queue::new(meta);
        
        assert!(!queue.is_finished_and_emitted());
        
        queue.state = FinishedState::FinishedAndEmitted;
        assert!(queue.is_finished_and_emitted());
    }

    #[test]
    fn test_queue_remove() {
        let meta = Metadata::new(());
        let mut queue: Queue<String, i32> = Queue::new(meta);
        
        queue.fifo.insert("key1".to_string(), 42);
        queue.fifo.insert("key2".to_string(), 84);
        assert_eq!(queue.fifo.len(), 2);
        
        queue.remove(&"key1".to_string());
        assert_eq!(queue.fifo.len(), 1);
        assert!(queue.fifo.contains_key("key2"));
        assert!(!queue.fifo.contains_key("key1"));
    }

    #[test]
    fn test_finished_state_not_finished() {
        let state = FinishedState::NotFinished;
        assert!(matches!(state, FinishedState::NotFinished));
    }

    #[test]
    fn test_finished_state_finished_but_not_emitted() {
        let meta = Metadata::new(());
        let state = FinishedState::FinishedButNotEmitted(meta);
        assert!(matches!(state, FinishedState::FinishedButNotEmitted(_)));
    }

    #[test]
    fn test_finished_state_finished_and_emitted() {
        let state = FinishedState::FinishedAndEmitted;
        assert!(matches!(state, FinishedState::FinishedAndEmitted));
    }

    #[test]
    fn test_finished_state_take_to_emit() {
        let meta = Metadata::new(());
        let mut state = FinishedState::FinishedButNotEmitted(meta);
        
        let result = state.take_to_emit();
        assert!(result.is_some());
        assert!(matches!(state, FinishedState::FinishedAndEmitted));
        
        // Should return None if called again
        let result2 = state.take_to_emit();
        assert!(result2.is_none());
        assert!(matches!(state, FinishedState::FinishedAndEmitted));
    }

    #[test]
    fn test_finished_state_take_to_emit_not_finished() {
        let mut state = FinishedState::NotFinished;
        
        let result = state.take_to_emit();
        assert!(result.is_none());
        assert!(matches!(state, FinishedState::NotFinished));
    }

    #[test]
    fn test_finished_state_take_to_emit_already_emitted() {
        let mut state = FinishedState::FinishedAndEmitted;
        
        let result = state.take_to_emit();
        assert!(result.is_none());
        assert!(matches!(state, FinishedState::FinishedAndEmitted));
    }

    #[test]
    fn test_queue_fifo_ordering() {
        let meta = Metadata::new(());
        let mut queue: Queue<String, i32> = Queue::new(meta);
        
        // Insert items in order
        queue.fifo.insert("first".to_string(), 1);
        queue.fifo.insert("second".to_string(), 2);
        queue.fifo.insert("third".to_string(), 3);
        
        // Should maintain insertion order
        let keys: Vec<String> = queue.fifo.keys().cloned().collect();
        assert_eq!(keys, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_queue_initial_metadata() {
        let meta = Metadata::new(());
        let mut queue: Queue<String, i32> = Queue::new(meta);
        
        assert!(queue.initial.is_some());
        
        // Take the initial metadata
        let taken = queue.initial.take();
        assert!(taken.is_some());
        assert!(queue.initial.is_none());
    }
}