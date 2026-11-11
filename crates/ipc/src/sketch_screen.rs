//! A `screen` node's layout tree as a client draws it: `screen.ts`'s `ScreenElement` in Rust, so the
//! native Bridge reads one without decoding JSON itself. **All of it or nothing**: a tree the checks in
//! `sketch::screen` find a problem in reads as `None`, as `readScreen` gives `undefined`.

use serde::Deserialize;

use crate::SceneNode;

/// `auto` takes its content, `fill` shares what is left, and a number is the weight of its share.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ScreenSize {
    Word(ScreenSizeWord),
    Weight(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenSizeWord {
    Auto,
    Fill,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenGap {
    None,
    #[default]
    Sm,
    Md,
    Lg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenAlign {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenChrome {
    Desktop,
    Browser,
    Phone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenVariant {
    Primary,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenField {
    #[default]
    Input,
    Textarea,
    Select,
    Checkbox,
    Toggle,
    Search,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenText {
    Heading,
    Title,
    #[default]
    Body,
    Caption,
}

/// A button or a bar's item: a bare string is its words, an object may carry a glyph with or without them.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ScreenAct {
    Words(String),
    Parts {
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        icon: Option<String>,
    },
}

impl ScreenAct {
    pub fn text(&self) -> Option<&str> {
        match self {
            ScreenAct::Words(text) => Some(text),
            ScreenAct::Parts { text, .. } => text.as_deref(),
        }
    }

    pub fn icon(&self) -> Option<&str> {
        match self {
            ScreenAct::Words(_) => None,
            ScreenAct::Parts { icon, .. } => icon.as_deref(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ScreenRow {
    pub text: String,
    #[serde(default)]
    pub meta: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenFrom {
    Me,
    Them,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ScreenMessage {
    pub from: ScreenFrom,
    pub text: String,
}

/// One element of a screen: what every kind carries, and its kind.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ScreenElement {
    /// Names the element when it carries one, so a reader and a walk can find it.
    #[serde(default)]
    pub id: Option<String>,
    /// Joins the scene's step order: ringed while the step plays, the rest of the screen stood back.
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub size: Option<ScreenSize>,
    #[serde(default)]
    pub align: Option<ScreenAlign>,
    #[serde(flatten)]
    pub part: ScreenPart,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ScreenPart {
    Row(ScreenLayout),
    Column(ScreenLayout),
    Grid {
        cols: u8,
        #[serde(flatten)]
        layout: ScreenLayout,
    },
    Window {
        chrome: ScreenChrome,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        url: Option<String>,
        children: Vec<ScreenElement>,
    },
    Nav {
        #[serde(default)]
        brand: Option<String>,
        #[serde(default)]
        items: Option<Vec<ScreenAct>>,
        #[serde(default)]
        active: Option<String>,
        #[serde(default)]
        actions: Option<Vec<ScreenAct>>,
        #[serde(default)]
        vertical: bool,
    },
    Panel {
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        icon: Option<String>,
        #[serde(default)]
        actions: Option<Vec<ScreenAct>>,
        #[serde(default)]
        gap: Option<ScreenGap>,
        #[serde(default)]
        pad: Option<ScreenGap>,
        #[serde(default)]
        children: Vec<ScreenElement>,
    },
    Tabs {
        items: Vec<String>,
        #[serde(default)]
        active: Option<String>,
    },
    List {
        rows: Vec<ScreenRow>,
    },
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Button {
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        variant: Option<ScreenVariant>,
        #[serde(default)]
        icon: Option<String>,
    },
    Field {
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        kind: ScreenField,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        placeholder: Option<String>,
    },
    Text {
        #[serde(default)]
        style: ScreenText,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        lines: Option<u8>,
    },
    Image {
        #[serde(default)]
        label: Option<String>,
    },
    Avatar {
        #[serde(default)]
        text: Option<String>,
    },
    Badge {
        text: String,
    },
    Divider,
    Spacer,
    Chat {
        messages: Vec<ScreenMessage>,
    },
    Modal {
        #[serde(default)]
        title: Option<String>,
        children: Vec<ScreenElement>,
    },
}

/// A row's, a column's or a grid's children and how they are spaced.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ScreenLayout {
    #[serde(default)]
    pub gap: Option<ScreenGap>,
    #[serde(default)]
    pub pad: Option<ScreenGap>,
    #[serde(default)]
    pub border: bool,
    pub children: Vec<ScreenElement>,
}

impl ScreenElement {
    /// What it holds (`childrenOf`).
    pub fn children(&self) -> &[ScreenElement] {
        match &self.part {
            ScreenPart::Row(layout) | ScreenPart::Column(layout) | ScreenPart::Grid { layout, .. } => &layout.children,
            ScreenPart::Window { children, .. } | ScreenPart::Panel { children, .. } | ScreenPart::Modal { children, .. } => children,
            _ => &[],
        }
    }

    /// Every step number it or anything under it carries (`screenSteps`).
    pub fn steps(&self) -> Vec<f64> {
        self.step.into_iter().chain(self.children().iter().flat_map(ScreenElement::steps)).collect()
    }

    /// Whether it, or anything under it, plays at `step` (`holdsStep`).
    pub fn holds(&self, step: f64) -> bool {
        self.step == Some(step) || self.children().iter().any(|child| child.holds(step))
    }

    /// A phone is as wide as one, and anything else as a desktop window (`screenWidth`).
    pub fn width(&self) -> f64 {
        if matches!(self.part, ScreenPart::Window { chrome: ScreenChrome::Phone, .. }) {
            390.
        } else {
            960.
        }
    }

    /// What it is called to somebody who cannot see it (`screenLabel`).
    pub fn label(&self) -> String {
        let named = match &self.part {
            ScreenPart::Button { text, icon, .. } => text.clone().or_else(|| icon.clone()),
            ScreenPart::Text { text, .. } | ScreenPart::Avatar { text } => text.clone(),
            ScreenPart::Badge { text } => Some(text.clone()),
            ScreenPart::Window { title, .. } | ScreenPart::Modal { title, .. } => title.clone(),
            ScreenPart::Panel { title, icon, .. } => title.clone().or_else(|| icon.clone()),
            ScreenPart::Field { label, .. } | ScreenPart::Image { label } => label.clone(),
            ScreenPart::Nav { brand, .. } => brand.clone(),
            _ => None,
        };
        named.or_else(|| self.id.clone()).unwrap_or_else(|| self.kind_word().to_string())
    }

    fn kind_word(&self) -> &'static str {
        match self.part {
            ScreenPart::Row(_) => "row",
            ScreenPart::Column(_) => "column",
            ScreenPart::Grid { .. } => "grid",
            ScreenPart::Window { .. } => "window",
            ScreenPart::Nav { .. } => "nav",
            ScreenPart::Panel { .. } => "panel",
            ScreenPart::Tabs { .. } => "tabs",
            ScreenPart::List { .. } => "list",
            ScreenPart::Table { .. } => "table",
            ScreenPart::Button { .. } => "button",
            ScreenPart::Field { .. } => "field",
            ScreenPart::Text { .. } => "text",
            ScreenPart::Image { .. } => "image",
            ScreenPart::Avatar { .. } => "avatar",
            ScreenPart::Badge { .. } => "badge",
            ScreenPart::Divider => "divider",
            ScreenPart::Spacer => "spacer",
            ScreenPart::Chat { .. } => "chat",
            ScreenPart::Modal { .. } => "modal",
        }
    }
}

impl SceneNode {
    /// A `screen` node's tree, where it is one and all of it reads. `None` otherwise.
    pub fn screen_tree(&self) -> Option<ScreenElement> {
        let tree = self.screen.as_ref()?;
        let mut found = Vec::new();
        crate::sketch::screen::problems(tree, "screen", &mut found);
        if !found.is_empty() {
            return None;
        }
        serde_json::from_value(tree.clone()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_reads_whole_and_one_with_a_glyph_nobody_has_does_not() {
        let node = |screen: serde_json::Value| SceneNode {
            id: "s".into(),
            kind: crate::SceneKind::Screen,
            x: 0.,
            y: 0.,
            w: None,
            h: None,
            title: None,
            body: None,
            icon: None,
            lang: None,
            wire: None,
            step: None,
            shape: None,
            color: None,
            html: None,
            css: None,
            styles: None,
            kept: None,
            screen: Some(screen),
        };
        let good = serde_json::json!({"type": "window", "chrome": "phone", "children": [
            {"type": "nav", "brand": "Shop", "items": ["Home", {"icon": "search"}], "active": "Home"},
            {"type": "column", "gap": "md", "children": [{"type": "text", "style": "heading", "text": "Sign in", "step": 1}, {"type": "button", "text": "Go", "variant": "primary", "size": "fill"}]}
        ]});
        let tree = node(good).screen_tree().expect("reads");
        assert_eq!(tree.width(), 390.);
        assert!(tree.holds(1.));
        assert_eq!(tree.steps(), vec![1.]);
        let bad = serde_json::json!({"type": "button", "icon": "no-such-glyph"});
        assert!(node(bad).screen_tree().is_none());
    }
}
