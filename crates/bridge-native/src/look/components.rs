//! The owner's theme, put into gpui-component's own `Theme`, and the fonts into gpui's text system.
//!
//! gpui-component draws its widgets (buttons, inputs, lists, tables) from `ThemeColor`, a fixed set
//! of roles. The tokens are a different set, so this is the one table that says which token plays
//! which role. A role with no token of its own keeps the library's value for the theme's tone
//! (the chart colours, for one); everything a person sees as a surface, an edge or a text is mapped.

use gpui::{px, App, Hsla};
use gpui_component::{Theme, ThemeMode};

use super::sources::{Tone, FONTS};
use super::{token, Look, Token};

/// Put `look` into gpui-component's theme. It draws with the first family of each font stack,
/// which is the Plex family `load_fonts` has put into the text system.
pub fn apply(look: &Look, cx: &mut App) {
    let mode = match look.tone() {
        Tone::Dark => ThemeMode::Dark,
        Tone::Light => ThemeMode::Light,
    };
    // Changing the mode loads the library's registered theme for it, and that load replaces the
    // colours; so the mode is set first and the colours after, in a second edit.
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        paint(&mut theme.colors, look);
        if let Some(family) = look.font_stack(token::FONT_SANS).into_iter().next() {
            theme.font_family = family;
        }
        if let Some(family) = look.font_stack(token::FONT_MONO).into_iter().next() {
            theme.mono_font_family = family;
        }
        theme.font_size = look.length(token::TEXT_BASE);
        theme.mono_font_size = look.length(token::TEXT_XS);
        theme.radius = look.length(token::RADIUS_MD);
        theme.radius_lg = look.length(token::RADIUS_LG);
        theme.transparent = gpui::transparent_black();
        // The focus ring is a token: its width and its colour, drawn by the library as a ring outside.
        theme.focus_ring = look.edge(token::FOCUS_RING).is_some_and(|(width, _)| width > px(0.0));
    });
}

/// Which token plays which role. Every line is one `ThemeColor` field and the token it reads.
fn paint(colors: &mut gpui_component::ThemeColor, look: &Look) {
    let c = |token: Token| -> Hsla { look.color(token) };

    // Ground and text.
    colors.background = c(token::BG_BASE);
    colors.foreground = c(token::FG_DEFAULT);
    colors.border = c(token::BORDER_DEFAULT);
    colors.input = c(token::BORDER_DEFAULT);
    colors.muted = c(token::BG_SUNKEN);
    colors.muted_foreground = c(token::FG_MUTED);
    colors.overlay = c(token::SCRIM);
    colors.window_border = c(token::BORDER_DEFAULT);
    colors.caret = c(token::FG_DEFAULT);
    colors.selection = c(token::ACCENT_MUTED);
    colors.ring = c(token::ACCENT);
    colors.link = c(token::ACCENT);
    colors.link_hover = c(token::ACCENT_HOVER);
    colors.link_active = c(token::ACCENT_HOVER);
    colors.drag_border = c(token::ACCENT);
    colors.drop_target = c(token::ACCENT_MUTED);

    // The accent is the primary action.
    colors.primary = c(token::ACCENT);
    colors.primary_hover = c(token::ACCENT_HOVER);
    colors.primary_active = c(token::ACCENT_HOVER);
    colors.primary_foreground = c(token::TEXT_ON_ACCENT);
    colors.button_primary = c(token::ACCENT);
    colors.button_primary_hover = c(token::ACCENT_HOVER);
    colors.button_primary_active = c(token::ACCENT_HOVER);
    colors.button_primary_foreground = c(token::TEXT_ON_ACCENT);

    // A raised surface is the secondary one; the hover ground is the library's accent.
    colors.secondary = c(token::BG_RAISED);
    colors.secondary_hover = c(token::BG_HOVER);
    colors.secondary_active = c(token::BG_HOVER);
    colors.secondary_foreground = c(token::FG_DEFAULT);
    colors.button = c(token::BG_RAISED);
    colors.button_hover = c(token::BG_HOVER);
    colors.button_active = c(token::BG_HOVER);
    colors.button_foreground = c(token::FG_DEFAULT);
    colors.button_secondary = c(token::BG_RAISED);
    colors.button_secondary_hover = c(token::BG_HOVER);
    colors.button_secondary_active = c(token::BG_HOVER);
    colors.button_secondary_foreground = c(token::FG_DEFAULT);
    colors.accent = c(token::BG_HOVER);
    colors.accent_foreground = c(token::FG_DEFAULT);

    // Floating things.
    colors.popover = c(token::BG_OVERLAY);
    colors.popover_foreground = c(token::FG_DEFAULT);
    colors.group_box = c(token::BG_RAISED);
    colors.group_box_foreground = c(token::FG_DEFAULT);
    colors.accordion = c(token::BG_BASE);

    // Status hues. The tokens carry one colour each and no hover or pressed variant, so those
    // three roles share it.
    let failed = c(token::STATUS_COMPLETED_FAILED);
    let success = c(token::STATUS_COMPLETED_SUCCESS);
    let warning = c(token::STATUS_AWAITING_REVIEW);
    let info = c(token::STATUS_RUNNING);
    let on = c(token::TEXT_ON_ACCENT);
    colors.danger = failed;
    colors.danger_hover = failed;
    colors.danger_active = failed;
    colors.danger_foreground = on;
    colors.button_danger = failed;
    colors.button_danger_hover = failed;
    colors.button_danger_active = failed;
    colors.button_danger_foreground = on;
    colors.success = success;
    colors.success_hover = success;
    colors.success_active = success;
    colors.success_foreground = on;
    colors.button_success = success;
    colors.button_success_hover = success;
    colors.button_success_active = success;
    colors.button_success_foreground = on;
    colors.warning = warning;
    colors.warning_hover = warning;
    colors.warning_active = warning;
    colors.warning_foreground = on;
    colors.button_warning = warning;
    colors.button_warning_hover = warning;
    colors.button_warning_active = warning;
    colors.button_warning_foreground = on;
    colors.info = info;
    colors.info_hover = info;
    colors.info_active = info;
    colors.info_foreground = on;
    colors.button_info = info;
    colors.button_info_hover = info;
    colors.button_info_active = info;
    colors.button_info_foreground = on;

    // Lists and tables: the row grounds the screens use.
    colors.list = c(token::SURFACE_CANVAS);
    colors.list_even = c(token::SURFACE_CANVAS);
    colors.list_head = c(token::SURFACE_WELL);
    colors.list_hover = c(token::SURFACE_ROW_HOVER);
    colors.list_active = c(token::SURFACE_ROW_SELECTED);
    colors.list_active_border = c(token::ACCENT);
    colors.table = c(token::SURFACE_CANVAS);
    colors.table_even = c(token::SURFACE_CANVAS);
    colors.table_head = c(token::SURFACE_WELL);
    colors.table_head_foreground = c(token::FG_MUTED);
    colors.table_foot = c(token::SURFACE_WELL);
    colors.table_foot_foreground = c(token::FG_MUTED);
    colors.table_hover = c(token::SURFACE_ROW_HOVER);
    colors.table_active = c(token::SURFACE_ROW_SELECTED);
    colors.table_active_border = c(token::ACCENT);
    colors.table_row_border = c(token::BORDER_SUBTLE);
    colors.description_list_label = c(token::SURFACE_WELL);
    colors.description_list_label_foreground = c(token::FG_MUTED);

    // Tabs.
    colors.tab = c(token::BG_BASE);
    colors.tab_active = c(token::BG_BASE);
    colors.tab_foreground = c(token::FG_MUTED);
    colors.tab_active_foreground = c(token::FG_DEFAULT);
    colors.tab_bar = c(token::BG_BASE);
    colors.tab_bar_segmented = c(token::BG_SUNKEN);

    // The sidebar, the title bar and the status bar.
    colors.sidebar = c(token::BG_BASE);
    colors.sidebar_foreground = c(token::FG_DEFAULT);
    colors.sidebar_border = c(token::BORDER_SUBTLE);
    colors.sidebar_accent = c(token::BG_HOVER);
    colors.sidebar_accent_foreground = c(token::FG_DEFAULT);
    colors.sidebar_primary = c(token::ACCENT);
    colors.sidebar_primary_foreground = c(token::TEXT_ON_ACCENT);
    colors.title_bar = c(token::BG_BASE);
    colors.title_bar_border = c(token::BORDER_SUBTLE);
    colors.status_bar = c(token::BG_BASE);
    colors.status_bar_border = c(token::BORDER_SUBTLE);

    // Small controls.
    colors.progress_bar = c(token::ACCENT);
    colors.slider_bar = c(token::ACCENT);
    colors.slider_thumb = c(token::FG_DEFAULT);
    colors.switch = c(token::BORDER_STRONG);
    colors.switch_thumb = c(token::FG_DEFAULT);
    colors.skeleton = c(token::BG_HOVER);
    colors.scrollbar = gpui::transparent_black();
    colors.scrollbar_thumb = c(token::BORDER_STRONG);
    colors.scrollbar_thumb_hover = c(token::FG_SUBTLE);

    // The library's named hues, from the nearest hue the tokens carry.
    colors.red = failed;
    colors.red_light = c(token::STATUS_COMPLETED_FAILED_BG);
    colors.green = success;
    colors.green_light = c(token::STATUS_COMPLETED_SUCCESS_BG);
    colors.blue = info;
    colors.blue_light = c(token::STATUS_RUNNING_BG);
    colors.yellow = warning;
    colors.yellow_light = c(token::STATUS_AWAITING_REVIEW_BG);
    colors.magenta = c(token::OPERATOR);
    colors.magenta_light = c(token::OPERATOR_MUTED);
    colors.cyan = c(token::STATUS_HANDED_IN);
    colors.cyan_light = c(token::STATUS_HANDED_IN_BG);
}

/// Put IBM Plex into gpui's text system, from the TrueType the build unpacked out of
/// `packages/tokens/fonts`. A face that will not load is logged and the system font stands in.
pub fn load_fonts(cx: &mut App) {
    let faces = FONTS.iter().map(|(_, bytes)| std::borrow::Cow::Borrowed(*bytes)).collect();
    if let Err(why) = cx.text_system().add_fonts(faces) {
        crate::log::warn("the Plex fonts did not load", [("why", why.to_string().into())]);
    }
}
