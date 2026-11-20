//! Two projects' values made one, as `merge_projects` makes them. `docs/concepts/project.md`, *Sharing with his
//! other Macs*.
//!
//! **A key is a field of `ValuesSet`**, so `look`, `reviews` and `tracker` each go whole to one project or the other.

use crate::ValuesSet;

macro_rules! each_key {
    ($($key:ident),*) => {
        /// The keys a project can set, by the name its `ValuesSet` field has.
        pub const VALUE_KEYS: &[&str] = &[$(stringify!($key)),*];

        impl ValuesSet {
            /// The keys both this set and `other` set, to different values: the rows a merge asks which to keep.
            /// A key one of them leaves out is never one.
            pub fn conflicts(&self, other: &ValuesSet) -> Vec<&'static str> {
                let mut keys = Vec::new();
                $(
                    if set(&self.$key) && set(&other.$key) && self.$key != other.$key {
                        keys.push(stringify!($key));
                    }
                )*
                keys
            }

            /// This set as `kept` and `gone` make one: **a key only `gone` sets is copied over, a key only `kept`
            /// sets stays, and a key both set differently takes `gone`'s only where `from_gone` names it.**
            pub fn merged(kept: &ValuesSet, gone: &ValuesSet, from_gone: &[String]) -> ValuesSet {
                let mut merged = kept.clone();
                $(
                    if set(&gone.$key) && (!set(&kept.$key) || from_gone.iter().any(|one| one == stringify!($key))) {
                        merged.$key = gone.$key.clone();
                    }
                )*
                merged
            }
        }
    };
}

each_key!(pr_mode, auto_merge, review_model, look, tracker, reviews, agent_starts, delivery, workflows_off, review_on_new_pr);

/// Whether a key says anything: an option that is present, or a list that is not empty.
fn set<T: IsSet>(held: &T) -> bool {
    held.is_set()
}

trait IsSet {
    fn is_set(&self) -> bool;
}

impl<T> IsSet for Option<T> {
    fn is_set(&self) -> bool {
        self.is_some()
    }
}

impl<T> IsSet for Vec<T> {
    fn is_set(&self) -> bool {
        !self.is_empty()
    }
}
