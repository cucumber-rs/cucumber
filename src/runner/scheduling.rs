//! Opt-in whole-application ordering, with native asynchronous concurrency.
//!
//! Random order reproduces the string-seed permutation of Cucumber.js 13.3's
//! knuth-shuffle-seeded 1.0.6 / seed-random 2.2.0. It is not cryptographic.

use super::{ThreadTimer, TimeoutClock};
use futures::future::BoxFuture;
use std::{
    collections::hash_map::RandomState,
    fmt,
    hash::BuildHasher as _,
    num::NonZeroUsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

/// Permutation of the complete selected initial case sequence.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ScenarioOrder {
    /// Preserve parser source order and expanded Examples row order.
    #[default]
    Defined,
    /// Reverse the entire selected case sequence.
    Reverse,
    /// Generate and report a decimal seed once per actual permutation.
    RandomGenerated,
    /// Reproduce the pinned JavaScript shuffle using the exact string seed.
    Random {
        /// Seed interpreted as UTF-16 code units, including non-BMP pairs.
        seed: String,
    },
}

/// Actual completed permutation, observed before case dispatch begins.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct OrderingRecord {
    /// Identity of this actual permutation; seeds themselves are not identities.
    pub run_id: u64,
    /// Requested order, distinguishing generated and explicit random seeds.
    pub requested_order: ScenarioOrder,
    /// Exact seed supplied to the pinned shuffle, or None for nonrandom orders.
    pub realized_seed: Option<String>,
    /// Number of selected expanded cases in the permuted global plan.
    pub case_count: usize,
    /// Time the permutation completed, before dispatch or report callbacks.
    pub at: SystemTime,
}
/// Shared snapshots of actual permutations, including sequential cloned reuse.
///
/// Creating options does not record a realization. Native scheduling without
/// explicit options does not write here. Records survive later cancellation.
#[derive(Clone, Debug, Default)]
pub struct OrderingStore(Arc<Mutex<Vec<OrderingRecord>>>);
impl OrderingStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns a snapshot of all actual completed permutations.
    #[must_use]
    pub fn records(&self) -> Vec<OrderingRecord> {
        self.0.lock().expect("ordering store poisoned").clone()
    }
    /// Returns the last completed permutation, if one has occurred.
    #[must_use]
    pub fn latest(&self) -> Option<OrderingRecord> {
        self.0.lock().expect("ordering store poisoned").last().cloned()
    }
    fn push(&self, record: OrderingRecord) {
        self.0.lock().expect("ordering store poisoned").push(record);
    }
}
type SeedReport = dyn Fn(&OrderingRecord);

/// Explicit buffered scheduling. Not enabled by the default runner.
#[derive(Clone)]
pub struct SchedulingOptions {
    pub(crate) order: ScenarioOrder,
    pub(crate) concurrency: Option<usize>,
    clock: Arc<dyn TimeoutClock>,
    pub(crate) assignment: Option<super::AssignmentPolicy>,
    ordering: OrderingStore,
    seed_report: Option<Arc<SeedReport>>,
}
impl fmt::Debug for SchedulingOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchedulingOptions")
            .field("order", &self.order)
            .field("concurrency", &self.concurrency)
            .field("clock", &self.clock)
            .field("assignment", &self.assignment)
            .field("ordering", &self.ordering)
            .finish_non_exhaustive()
    }
}
impl Default for SchedulingOptions {
    fn default() -> Self {
        Self {
            order: ScenarioOrder::Defined,
            concurrency: None,
            clock: Arc::new(ThreadTimer),
            assignment: None,
            ordering: OrderingStore::new(),
            seed_report: None,
        }
    }
}
impl SchedulingOptions {
    /// Selects a global permutation; concurrency inherits the native runner.
    #[must_use]
    pub fn new(order: ScenarioOrder) -> Self {
        Self { order, ..Self::default() }
    }
    /// Shares actual ordering realizations with the application.
    #[must_use]
    pub fn with_ordering_store(mut self, store: OrderingStore) -> Self {
        self.ordering = store;
        self
    }
    /// Returns the store shared by these options and their clones.
    #[must_use]
    pub fn ordering_store(&self) -> OrderingStore {
        self.ordering.clone()
    }
    /// Replaces generated-seed stderr reporting with an application callback.
    ///
    /// Runs once after storing the completed permutation and before dispatch.
    /// Explicit seeds (including an empty string) do not invoke this callback.
    #[must_use]
    pub fn on_generated_seed<F>(mut self, report: F) -> Self
    where
        F: Fn(&OrderingRecord) + 'static,
    {
        self.seed_report = Some(Arc::new(report));
        self
    }
    /// Adds candidate/running-set eligibility to this explicit ordered dispatcher.
    /// Mandatory Serial barriers remain in force and idle fallback is observable.
    #[must_use]
    pub fn with_assignment(mut self, policy: super::AssignmentPolicy) -> Self {
        self.assignment = Some(policy);
        self
    }
    /// Sets a strictly positive native logical-case limit.
    #[must_use]
    pub fn concurrency(mut self, limit: NonZeroUsize) -> Self {
        self.concurrency = Some(limit.get());
        self
    }
    /// Uses JavaScript parallel-number semantics: zero means serial, not no work.
    /// This still uses native async cases, not worker processes.
    #[must_use]
    pub fn parallel(mut self, parallel: usize) -> Self {
        self.concurrency = Some(parallel.max(1));
        self
    }
    /// Supplies runtime-independent retry delays, independently of callback timeouts.
    #[must_use]
    pub fn with_retry_clock(mut self, clock: Arc<dyn TimeoutClock>) -> Self {
        self.clock = clock;
        self
    }
    pub(crate) fn delay(&self, duration: Duration) -> BoxFuture<'static, ()> {
        self.clock.sleep(duration)
    }
    pub(crate) fn order<T>(&self, cases: &mut [T]) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let run_id = NEXT.fetch_add(1, Ordering::Relaxed);
        let seed = match &self.order {
            ScenarioOrder::RandomGenerated => {
                // Native noncryptographic entropy; replay uses the exact realized
                // decimal, not a claim to match JavaScript's entropy generator.
                Some(
                    (RandomState::new().hash_one(run_id) % 1_000_000)
                        .to_string(),
                )
            }
            ScenarioOrder::Random { seed } => Some(seed.clone()),
            ScenarioOrder::Defined | ScenarioOrder::Reverse => None,
        };
        if let Some(seed) = &seed {
            let mut random = SeedRandom::new(seed);
            for remaining in (1..=cases.len()).rev() {
                let index = (random.next() * remaining as f64).floor() as usize;
                cases.swap(remaining - 1, index);
            }
        } else if self.order == ScenarioOrder::Reverse {
            cases.reverse();
        }
        let record = OrderingRecord {
            run_id,
            requested_order: self.order.clone(),
            realized_seed: seed,
            case_count: cases.len(),
            at: SystemTime::now(),
        };
        self.ordering.push(record.clone());
        if self.order == ScenarioOrder::RandomGenerated {
            if let Some(report) = &self.seed_report {
                report(&record);
            } else {
                eprintln!(
                    "Random order using seed: {}",
                    record
                        .realized_seed
                        .as_deref()
                        .expect("generated seed realized")
                );
            }
        }
    }
}

// Attributed compatibility port of knuth-shuffle-seeded 1.0.6 (Apache-2.0),
// Copyright 2013 AJ O'Neal and 2015 Tiancheng "Timothy" Gu,
// and seed-random 2.2.0 (MIT modifications / BSD original).
// These are modified Rust translations. Full original source/licenses and
// provenance: tests/fixtures/shuffle37. The seed-random notices follow below.
// Keep the precise floating-point algorithm and UTF-16 mixing; changing the
// random generator changes cross-language replay for the same seed.
struct SeedRandom {
    s: [u8; 256],
    i: usize,
    j: usize,
}
impl SeedRandom {
    fn new(seed: &str) -> Self {
        let mut key = Vec::<u8>::new();
        let mut smear = 0u32;
        for (j, unit) in seed.encode_utf16().enumerate() {
            let index = j & 255;
            if key.len() <= index {
                key.resize(index + 1, 0);
            }
            smear ^= u32::from(key[index]) * 19;
            key[index] = ((smear + u32::from(unit)) & 255) as u8;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut rng = Self { s: std::array::from_fn(|i| i as u8), i: 0, j: 0 };
        let mut j = 0;
        for i in 0..256 {
            j = (j + usize::from(key[i % key.len()]) + usize::from(rng.s[i]))
                & 255;
            rng.s.swap(i, j);
        }
        for _ in 0..256 {
            let _ = rng.byte();
        }
        rng
    }
    fn byte(&mut self) -> u8 {
        self.i = (self.i + 1) & 255;
        let t = self.s[self.i];
        self.j = (self.j + usize::from(t)) & 255;
        self.s.swap(self.i, self.j);
        self.s[(usize::from(self.s[self.i]) + usize::from(t)) & 255]
    }
    fn next(&mut self) -> f64 {
        let mut n = 0.0;
        for _ in 0..6 {
            n = n * 256.0 + f64::from(self.byte());
        }
        let mut d = 281_474_976_710_656.0; // 256^6
        let mut x = 0u32;
        while n < 4_503_599_627_370_496.0 {
            n = (n + f64::from(x)) * 256.0;
            d *= 256.0;
            x = u32::from(self.byte());
        }
        while n >= 9_007_199_254_740_992.0 {
            n /= 2.0;
            d /= 2.0;
            x >>= 1;
        }
        (n + f64::from(x)) / d
    }
}

/*
LICENSE for modifications of JavaScript Library by Forbes Lindesay (MIT):

Copyright (c) 2013 Forbes Lindesay

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

LICENSE for original seedrandom javascript file (BSD):

Copyright 2013 David Bau, all rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

  1. Redistributions of source code must retain the above copyright
     notice, this list of conditions and the following disclaimer.

  2. Redistributions in binary form must reproduce the above copyright
     notice, this list of conditions and the following disclaimer in the
     documentation and/or other materials provided with the distribution.

  3. Neither the name of this module nor the names of its contributors may
     be used to endorse or promote products derived from this software
     without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
*/
