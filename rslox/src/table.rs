use std::hash::Hasher;

use rustc_hash::FxHasher;
use tracing::{debug, instrument};

use crate::value::{LoxStr, Value};

#[derive(Debug, Clone)]
pub struct Entry {
    pub(crate) key: LoxStr,
    pub(crate) val: Value,
}

impl Entry {
    const TOMBSTONE: Self = Self {
        key: LoxStr::EMPTY,
        val: Value::Nil,
    };

    pub fn is_tombstone(&self) -> bool {
        self.key.str() == Self::TOMBSTONE.key.str() && self.val == Self::TOMBSTONE.val
    }
}

#[derive(Debug, Default, Clone)]
pub struct Table {
    count: u32,
    pub entries: Box<[Option<Entry>]>,
}

impl Table {
    const SEED: usize = 0xf25d_328e_f414_5735;
    const MAX_LOAD: f64 = 0.75;

    pub fn new() -> Self {
        Self {
            count: 0,
            entries: Box::default(),
        }
    }

    pub fn hash(s: &str) -> u64 {
        let mut hasher = FxHasher::with_seed(Table::SEED);
        hasher.write(s.as_bytes());
        hasher.finish()
    }

    fn find_idx(&self, key: &'static str) -> usize {
        let capacity = self.entries.len();

        let mut idx = Self::hash(key) as usize & (capacity - 1);
        let mut tombstone_idx = usize::MAX;

        loop {
            {
                // strings are interned so a pointer comparison should work
                // the indexing operations can't panic due to % capacity
                // we can't assign &mut self.entries[idx] to a variable because otherwise
                // Rust gets mad about borrowing through the loop. I could avoid that
                // with a pointer cast but the compiler should catch it anyway.
                if self.entries[idx].as_ref().is_none_or(|x| {
                    if x.val == Entry::TOMBSTONE.val {
                        if tombstone_idx == usize::MAX {
                            tombstone_idx = idx;
                        }
                        false
                    } else {
                        x.key.str() == key
                    }
                }) {
                    // return the first tombstone instead of the first empty if we passed a tombstone
                    let i = if tombstone_idx != usize::MAX {
                        // debug!("Fell back to tombstone for key {key} at index {idx}");
                        tombstone_idx
                    } else {
                        // debug!("Found key {key} at index {idx}");
                        idx
                    };

                    return i;
                }
            }

            idx = (idx + 1) & (capacity - 1);
        }
    }

    fn find_mut(&mut self, key: &'static str) -> &mut Option<Entry> {
        &mut self.entries[self.find_idx(key)]
    }

    fn find(&self, key: &'static str) -> &Option<Entry> {
        &self.entries[self.find_idx(key)]
    }

    pub fn insert(&mut self, key: LoxStr, val: Value) -> bool {
        if (self.count + 1) as usize > (self.entries.len() as f64 * Self::MAX_LOAD) as usize {
            let new_len = if self.entries.is_empty() {
                8
            } else {
                self.entries.len() * 2
            };

            let mut new = vec![None; new_len].into_boxed_slice();
            self.count = 0;

            for entry in self.entries.iter().flatten() {
                if !entry.is_tombstone() {
                    self.count += 1;
                    let mut idx = Self::hash(entry.key.str()) as usize & (new.len() - 1);
                    loop {
                        match &mut new[idx] {
                            Some(_) => idx = (idx + 1) & (new.len() - 1),
                            x => {
                                *x = Some(entry.clone());
                                break;
                            }
                        }
                    }
                }
            }
            self.entries = new;
        }

        let entry = self.find_mut(key.str());

        if let Some(e) = entry {
            assert!(e.val == Entry::TOMBSTONE.val || e.key.str() == key.str());
        }

        let new = entry.is_none();
        // debug!("overwriting {entry:?} with ({key}, {val})");

        *entry = Some(Entry { key, val });

        if new {
            self.count += 1;
        }

        // debug!("After insert (k:{},v:{}): {:#?}", key, val, self);
        new
    }

    pub fn get(&mut self, key: &'static str) -> Option<&mut Value> {
        if self.count == 0 {
            return None;
        }

        self.find_mut(key).as_mut().map(|x| &mut x.val)
    }

    pub fn get_ref(&self, key: &'static str) -> Option<&Value> {
        if self.count == 0 {
            return None;
        }

        self.find(key).as_ref().map(|x| &x.val)
    }

    pub fn remove(&mut self, key: &'static str) -> bool {
        if self.count == 0 {
            return false;
        }

        match self.find_mut(key) {
            Some(e) => {
                *e = Entry::TOMBSTONE;
                true
            }
            None => false,
        }
    }

    pub fn get_key(&self, key: &str) -> Option<LoxStr> {
        let capacity = self.entries.len();

        if capacity == 0 {
            return None;
        }

        let mut idx = Self::hash(key) as usize & (capacity - 1);

        loop {
            {
                match &self.entries[idx] {
                    Some(e) => {
                        if e.key.str() == key {
                            return Some(e.key);
                        }
                    }
                    None => {
                        return None;
                    }
                }
            }

            idx = (idx + 1) & (capacity - 1);
        }
    }

    pub fn clear(&mut self) {
        self.entries.iter_mut().for_each(|x| *x = None);
    }
}
