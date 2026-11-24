//! Where the System block is zoomed to: the project, a part of it, or one feature's path through that part. Nothing here
//! touches the screen or the wire.

/// One level of the map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Level {
    Project,
    Part { id: String, name: String },
    Feature { slug: String, title: String },
}

/// What a crumb at the top of the block says and where a press on it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crumb {
    pub words: String,
    /// The depth, from 1, a press steps out to.
    pub depth: usize,
}

/// The levels from the project down to where the block is zoomed. It always holds the project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trail {
    steps: Vec<Level>,
}

impl Default for Trail {
    fn default() -> Self {
        Trail { steps: vec![Level::Project] }
    }
}

impl Trail {
    /// One, two or three: the level the map on screen is at.
    pub fn depth(&self) -> usize {
        self.steps.len()
    }

    pub fn here(&self) -> &Level {
        self.steps.last().unwrap_or(&Level::Project)
    }

    /// The part the map is zoomed into, on level two and on level three, which stands inside it.
    pub fn part(&self) -> Option<&str> {
        self.steps.iter().find_map(|step| match step {
            Level::Part { id, .. } => Some(id.as_str()),
            _ => None,
        })
    }

    /// The feature whose path is drawn, on level three.
    pub fn feature(&self) -> Option<&str> {
        match self.here() {
            Level::Feature { slug, .. } => Some(slug),
            _ => None,
        }
    }

    /// A part is zoomed into, from level one only.
    pub fn zoom_part(&mut self, id: &str, name: &str) -> bool {
        if self.depth() != 1 {
            return false;
        }
        self.steps.push(Level::Part { id: id.to_string(), name: name.to_string() });
        true
    }

    /// A feature's path is zoomed into, from level two only.
    pub fn zoom_feature(&mut self, slug: &str, title: &str) -> bool {
        if self.depth() != 2 {
            return false;
        }
        self.steps.push(Level::Feature { slug: slug.to_string(), title: title.to_string() });
        true
    }

    /// One level out. At level one there is none.
    pub fn up(&mut self) -> bool {
        if self.depth() == 1 {
            return false;
        }
        self.steps.pop();
        true
    }

    /// Out to `depth`, which is a crumb's press. A depth that is not shallower than this one changes nothing.
    pub fn out_to(&mut self, depth: usize) -> bool {
        if depth == 0 || depth >= self.depth() {
            return false;
        }
        self.steps.truncate(depth);
        true
    }

    /// The path as the crumbs say it, the project first.
    pub fn crumbs(&self, project: &str) -> Vec<Crumb> {
        self.steps
            .iter()
            .enumerate()
            .map(|(at, step)| Crumb {
                words: match step {
                    Level::Project => project.to_string(),
                    Level::Part { name, .. } => name.clone(),
                    Level::Feature { title, .. } => title.clone(),
                },
                depth: at + 1,
            })
            .collect()
    }
}

/// What the block does with Esc: step out a level, or at level one put a grown block back in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    Out,
    Shrink,
    /// The block is in its place at level one: Esc is not the block's.
    Nothing,
}

pub fn leave(trail: &Trail, grown: bool) -> Leave {
    if trail.depth() > 1 {
        Leave::Out
    } else if grown {
        Leave::Shrink
    } else {
        Leave::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_is_zoomed_into_from_the_project_and_a_feature_from_the_part() {
        let mut trail = Trail::default();
        assert!(!trail.zoom_feature("dashboard", "Dashboard"), "a feature's path is drawn inside a part");
        assert!(trail.zoom_part("bridge", "Bridge"));
        assert!(!trail.zoom_part("fleet", "Fleet"), "no part inside a part");
        assert_eq!((trail.depth(), trail.part()), (2, Some("bridge")));
        assert!(trail.zoom_feature("dashboard", "Dashboard"));
        assert_eq!((trail.depth(), trail.part(), trail.feature()), (3, Some("bridge"), Some("dashboard")));
        assert!(!trail.zoom_feature("jobs", "Jobs"));
    }

    #[test]
    fn esc_steps_out_one_level_and_a_crumb_steps_out_to_its_own() {
        let mut trail = Trail::default();
        trail.zoom_part("bridge", "Bridge");
        trail.zoom_feature("dashboard", "Dashboard");
        assert!(trail.up());
        assert_eq!(trail.here(), &Level::Part { id: "bridge".into(), name: "Bridge".into() });
        trail.zoom_feature("dashboard", "Dashboard");
        assert!(trail.out_to(1));
        assert_eq!(trail.depth(), 1);
        assert!(!trail.up(), "the project has nothing above it");
        assert!(!trail.out_to(1), "a crumb that is where it is changes nothing");
    }

    #[test]
    fn the_crumbs_say_the_project_then_each_level_with_the_depth_a_press_goes_to() {
        let mut trail = Trail::default();
        trail.zoom_part("bridge", "Bridge");
        trail.zoom_feature("dashboard", "Dashboard");
        let crumbs: Vec<(String, usize)> = trail.crumbs("armada").into_iter().map(|one| (one.words, one.depth)).collect();
        assert_eq!(crumbs, [("armada".to_string(), 1), ("Bridge".to_string(), 2), ("Dashboard".to_string(), 3)]);
    }

    #[test]
    fn esc_at_level_one_puts_a_grown_block_back_and_is_not_the_blocks_when_it_is_in_place() {
        let mut trail = Trail::default();
        assert_eq!(leave(&trail, false), Leave::Nothing);
        assert_eq!(leave(&trail, true), Leave::Shrink);
        trail.zoom_part("bridge", "Bridge");
        assert_eq!(leave(&trail, true), Leave::Out);
    }
}
