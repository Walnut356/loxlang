use std::hash::Hasher;

use rustc_hash::FxHasher;

use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Entry {
    pub(crate) key: &'static str,
    pub(crate) val: Value,
}

impl Entry {
    const TOMBSTONE: Self = Self {
        key: "",
        val: Value::Nil,
    };

    pub fn is_tombstone(&self) -> bool {
        self.val == Self::TOMBSTONE.val
    }
}

#[derive(Debug, Default)]
pub struct Table {
    count: u32,
    // Option entry is the same size as entry so there's no need for tombstone values
    pub entries: Vec<Option<Entry>>,
}

impl Table {
    const SEED: usize = 0xf25d328ef4145735;
    const MAX_LOAD: f64 = 0.75;

    pub fn new() -> Self {
        Self {
            count: 0,
            entries: Vec::default(),
        }
    }

    pub fn hash(s: &str) -> u64 {
        let mut hasher = FxHasher::with_seed(Table::SEED);
        hasher.write(s.as_bytes());
        hasher.finish()
    }

    fn find_mut(&mut self, key: &'static str) -> &mut Option<Entry> {
        let capacity = self.entries.len();

        // unsafe { std::hint::assert_unchecked(capacity != 0) };
        let mut idx = Self::hash(key) as usize % capacity;
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
                        tombstone_idx = idx;
                        false
                    } else {
                        true
                    }
                }) {
                    // return the first tombstone instead of the first empty if we passed a tombstone
                    let i = if tombstone_idx != usize::MAX {
                        tombstone_idx
                    } else {
                        idx
                    };

                    return &mut self.entries[i];
                }
            }

            idx = (idx + 1) % capacity;
        }
    }

    pub fn insert(&mut self, key: &'static str, val: Value) -> bool {
        if (self.count + 1) as usize > (self.entries.len() as f64 * Self::MAX_LOAD) as usize {
            let new_len = if self.entries.is_empty() {
                8
            } else {
                self.entries.len() * 2
            };

            let mut new = vec![None; new_len];
            self.count = 0;

            for entry in self.entries.iter().flatten() {
                if !entry.is_tombstone() {
                    self.count += 1;
                    let idx = Self::hash(entry.key) as usize % new.len();
                    new[idx] = Some(entry.clone());
                }
            }
            self.entries.resize_with(new_len, || None);
        }

        let entry = self.find_mut(key);
        let new = entry.is_none();

        *entry = Some(Entry { key, val });

        if new {
            self.count += 1;
        }

        new
    }

    pub fn get(&mut self, key: &'static str) -> Option<&mut Value> {
        if self.count == 0 {
            return None;
        }

        self.find_mut(key).as_mut().map(|x| &mut x.val)
    }

    pub fn remove(&mut self, key: &'static str) -> bool {
        if self.count == 0 {
            return false;
        }

        let entry = self.find_mut(key);

        if entry.is_none() {
            return false;
        }

        entry.replace(Entry {
            key: "",
            val: Value::Bool(true),
        });

        true
    }

    pub fn get_key(&self, key: &str) -> Option<&'static str> {
        let capacity = self.entries.len();

        if capacity == 0 {
            return None;
        }

        let mut idx = Self::hash(key) as usize % capacity;

        loop {
            {
                match &self.entries[idx] {
                    Some(e) => {
                        if e.key == key {
                            return Some(e.key);
                        }
                    }
                    None => {
                        return None;
                    }
                }
            }

            idx = (idx + 1) % capacity;
        }
    }
}
