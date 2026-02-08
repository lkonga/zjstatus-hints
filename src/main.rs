use ansi_term::{ANSIString, ANSIStrings, Colour, Style};
use std::collections::BTreeMap;
use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::actions::SearchDirection;
use zellij_tile::prelude::*;

/// Theme colors for the hints plugin
/// Supports hex color strings (e.g., "#45475a") from plugin configuration
#[derive(Clone)]
struct ThemeColors {
    key_bg: Colour,
    key_fg: Colour,
    desc_bg: Colour,
    desc_fg: Colour,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            key_bg: parse_hex_color(DEFAULT_KEY_BG),
            key_fg: parse_hex_color(DEFAULT_KEY_FG),
            desc_bg: parse_hex_color(DEFAULT_DESC_BG),
            desc_fg: parse_hex_color(DEFAULT_DESC_FG),
        }
    }
}

impl ThemeColors {
    fn from_config(configuration: &BTreeMap<String, String>) -> Self {
        Self {
            key_bg: configuration
                .get("key_bg")
                .map(|s| parse_hex_color(s))
                .unwrap_or_else(|| parse_hex_color(DEFAULT_KEY_BG)),
            key_fg: configuration
                .get("key_fg")
                .map(|s| parse_hex_color(s))
                .unwrap_or_else(|| parse_hex_color(DEFAULT_KEY_FG)),
            desc_bg: configuration
                .get("desc_bg")
                .map(|s| parse_hex_color(s))
                .unwrap_or_else(|| parse_hex_color(DEFAULT_DESC_BG)),
            desc_fg: configuration
                .get("desc_fg")
                .map(|s| parse_hex_color(s))
                .unwrap_or_else(|| parse_hex_color(DEFAULT_DESC_FG)),
        }
    }
}

/// Parse a hex color string (e.g., "#45475a" or "45475a") into an ansi_term Colour
fn parse_hex_color(hex: &str) -> Colour {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Colour::RGB(r, g, b);
        }
    }
    // Fallback to default key_bg color if parsing fails
    Colour::RGB(0x45, 0x47, 0x5a)
}

#[derive(Default)]
struct State {
    initialized: bool,
    pipe_name: String,
    mode_info: ModeInfo,
    base_mode_is_locked: bool,
    max_length: usize,
    overflow_str: String,
    hide_in_base_mode: bool,
    // Theme colors for styling
    theme: ThemeColors,
}

register_plugin!(State);

const TO_NORMAL: Action = Action::SwitchToMode(InputMode::Normal);

const PLUGIN_SESSION_MANAGER: &str = "session-manager";
const PLUGIN_CONFIGURATION: &str = "configuration";
const PLUGIN_MANAGER: &str = "plugin-manager";
const PLUGIN_ABOUT: &str = "zellij:about";

const KEY_PATTERNS_NO_SEPARATOR: &[&str] = &["HJKL", "hjkl", "←↓↑→", "←→", "↓↑", "[]"];

const DEFAULT_MAX_LENGTH: usize = 0;
const DEFAULT_OVERFLOW_STR: &str = "...";
const DEFAULT_PIPE_NAME: &str = "zjstatus_hints";

// Default Catppuccin Frappe colors
const DEFAULT_KEY_BG: &str = "#45475a"; // surface1
const DEFAULT_KEY_FG: &str = "#c6d0f5"; // text
const DEFAULT_DESC_BG: &str = "#313244"; // surface0
const DEFAULT_DESC_FG: &str = "#bac2de"; // text2

type ActionLabel = (Action, &'static str);
type ActionSequenceLabel = (&'static [Action], &'static str);

const NORMAL_MODE_ACTIONS: &[ActionLabel] = &[
    (Action::SwitchToMode(InputMode::Pane), "pane"),
    (Action::SwitchToMode(InputMode::Tab), "tab"),
    (Action::SwitchToMode(InputMode::Resize), "resize"),
    (Action::SwitchToMode(InputMode::Move), "move"),
    (Action::SwitchToMode(InputMode::Scroll), "scroll"),
    (Action::SwitchToMode(InputMode::Search), "search"),
    (Action::SwitchToMode(InputMode::Session), "session"),
    (Action::Quit, "quit"),
];

// Tmux mode actions - gateway modes accessible from Tmux mode
const TMUX_MODE_ACTIONS: &[ActionLabel] = &[
    (Action::SwitchToMode(InputMode::Pane), "pane"),
    (Action::SwitchToMode(InputMode::Tab), "tab"),
    (Action::SwitchToMode(InputMode::Resize), "resize"),
    (Action::SwitchToMode(InputMode::Move), "move"),
    (Action::SwitchToMode(InputMode::Scroll), "scroll"),
    (Action::SwitchToMode(InputMode::Session), "session"),
    (Action::SwitchToMode(InputMode::Locked), "lock"),
];

// Tmux mode action sequences - common tmux-style operations
const TMUX_MODE_ACTION_SEQUENCES: &[ActionSequenceLabel] = &[
    (&[Action::NewPane(None, None, false), TO_NORMAL], "new"),
    (
        &[
            Action::NewPane(Some(Direction::Right), None, false),
            TO_NORMAL,
        ],
        "split right",
    ),
    (
        &[
            Action::NewPane(Some(Direction::Down), None, false),
            TO_NORMAL,
        ],
        "split down",
    ),
    (&[Action::CloseFocus, TO_NORMAL], "close"),
    (&[Action::ToggleFocusFullscreen, TO_NORMAL], "fullscreen"),
    (&[Action::Detach], "detach"),
];

const PANE_MODE_ACTION_SEQUENCES: &[ActionSequenceLabel] = &[
    (&[Action::NewPane(None, None, false), TO_NORMAL], "new"),
    (&[Action::CloseFocus, TO_NORMAL], "close"),
    (&[Action::ToggleFocusFullscreen, TO_NORMAL], "fullscreen"),
    (&[Action::ToggleFloatingPanes, TO_NORMAL], "float"),
    (&[Action::TogglePaneEmbedOrFloating, TO_NORMAL], "embed"),
    (
        &[
            Action::NewPane(Some(Direction::Right), None, false),
            TO_NORMAL,
        ],
        "split right",
    ),
    (
        &[
            Action::NewPane(Some(Direction::Down), None, false),
            TO_NORMAL,
        ],
        "split down",
    ),
];

const TAB_MODE_ACTION_SEQUENCES: &[ActionSequenceLabel] = &[
    (
        &[
            Action::NewTab(None, vec![], None, None, None, true),
            TO_NORMAL,
        ],
        "new",
    ),
    (&[Action::CloseTab, TO_NORMAL], "close"),
    (&[Action::BreakPane, TO_NORMAL], "break pane"),
    (&[Action::ToggleActiveSyncTab, TO_NORMAL], "sync"),
];

fn get_common_modifiers(mut key_bindings: Vec<&KeyWithModifier>) -> Vec<KeyModifier> {
    if key_bindings.is_empty() {
        return vec![];
    }
    let mut common_modifiers = key_bindings.pop().unwrap().key_modifiers.clone();
    for key in key_bindings {
        common_modifiers = common_modifiers
            .intersection(&key.key_modifiers)
            .cloned()
            .collect();
    }
    common_modifiers.into_iter().collect()
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.initialized = false;

        // TODO: configuration validation
        self.max_length = configuration
            .get("max_length")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_LENGTH);
        self.overflow_str = configuration
            .get("overflow_str")
            .cloned()
            .unwrap_or_else(|| DEFAULT_OVERFLOW_STR.to_string());
        self.pipe_name = configuration
            .get("pipe_name")
            .cloned()
            .unwrap_or_else(|| DEFAULT_PIPE_NAME.to_string());
        self.hide_in_base_mode = configuration
            .get("hide_in_base_mode")
            .map(|s| s.to_lowercase().parse::<bool>().unwrap_or(false))
            .unwrap_or(false);

        // Theme colors from configuration (defaults to Catppuccin Frappe)
        self.theme = ThemeColors::from_config(&configuration);

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::MessageAndLaunchOtherPlugins,
        ]);

        set_selectable(false);
        subscribe(&[EventType::ModeUpdate, EventType::SessionUpdate]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = !self.initialized;
        if let Event::ModeUpdate(mode_info) = event {
            if self.mode_info != mode_info {
                should_render = true;
            }
            self.mode_info = mode_info;
            self.base_mode_is_locked = self.mode_info.base_mode == Some(InputMode::Locked);
        };
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        let mode_info = &self.mode_info;
        let output = if !(self.hide_in_base_mode && Some(mode_info.mode) == mode_info.base_mode) {
            let keymap = get_keymap_for_mode(mode_info);
            // Pass available width for responsive hint display
            let parts = render_hints_for_mode(mode_info.mode, &keymap, &self.theme, cols);

            let ansi_strings = ANSIStrings(&parts);
            let formatted = format!(" {}", ansi_strings);

            let visible_len = calculate_visible_length(&formatted);
            if self.max_length > 0 && visible_len > self.max_length {
                truncate_ansi_string(&formatted, &self.overflow_str, self.max_length)
            } else {
                formatted.to_string()
            }
        } else {
            String::new()
        };

        // HACK: Because we're not sure when zjstatus will be ready to receive messages,
        // we'll repeatedly send messages until the user has switched to a different mode,
        // at which point we'll assume that zjstatus has been initialized. The render function
        // does not seem to be called too frequently, so this should be fine.
        if !output.is_empty() && Some(mode_info.mode) != mode_info.base_mode {
            self.initialized = true;
        }

        pipe_message_to_plugin(MessageToPlugin::new("pipe").with_payload(format!(
            "zjstatus::pipe::pipe_{}::{}",
            self.pipe_name, output
        )));
        print!("{}", output);
    }
}

struct AnsiParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> AnsiParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            chars: text.chars().peekable(),
        }
    }

    fn next_segment(&mut self) -> Option<AnsiSegment> {
        let ch = self.chars.next()?;

        if ch == '\x1b' {
            let mut escape_seq = String::from(ch);
            for escape_ch in self.chars.by_ref() {
                escape_seq.push(escape_ch);
                if escape_ch == 'm' {
                    break;
                }
            }
            Some(AnsiSegment::EscapeSequence(escape_seq))
        } else {
            Some(AnsiSegment::VisibleChar(ch))
        }
    }
}

enum AnsiSegment {
    EscapeSequence(String),
    VisibleChar(char),
}

fn calculate_visible_length(text: &str) -> usize {
    let mut parser = AnsiParser::new(text);
    let mut len = 0;

    while let Some(segment) = parser.next_segment() {
        if matches!(segment, AnsiSegment::VisibleChar(_)) {
            len += 1;
        }
    }

    len
}

fn truncate_ansi_string(text: &str, overflow_str: &str, max_len: usize) -> String {
    let visible_len = calculate_visible_length(text);
    let overflow_len = overflow_str.len();

    if visible_len <= max_len {
        return text.to_string();
    }

    if max_len <= overflow_len {
        return overflow_str.to_string();
    }

    let target_len = max_len - overflow_len;
    let mut result = String::new();
    let mut visible_count = 0;
    let mut parser = AnsiParser::new(text);

    while let Some(segment) = parser.next_segment() {
        match segment {
            AnsiSegment::EscapeSequence(seq) => {
                result.push_str(&seq);
            }
            AnsiSegment::VisibleChar(ch) => {
                if visible_count >= target_len {
                    break;
                }
                result.push(ch);
                visible_count += 1;
            }
        }
    }

    result.push_str(overflow_str);
    result
}

fn find_keys_for_actions(
    keymap: &[(KeyWithModifier, Vec<Action>)],
    target_actions: &[Action],
    exact_match: bool,
) -> Vec<KeyWithModifier> {
    keymap
        .iter()
        .filter_map(|(key, key_actions)| {
            if exact_match {
                let matching = key_actions
                    .iter()
                    .zip(target_actions)
                    .filter(|(a, b)| a.shallow_eq(b))
                    .count();
                if matching == key_actions.len() && matching == target_actions.len() {
                    Some(key.clone())
                } else {
                    None
                }
            } else {
                // FIX: Use shallow_eq for first action comparison instead of ==
                // This fixes Pane mode which was showing "00" due to failed matching
                match (key_actions.first(), target_actions.first()) {
                    (Some(key_action), Some(target_action))
                        if key_action.shallow_eq(target_action) =>
                    {
                        Some(key.clone())
                    }
                    _ => None,
                }
            }
        })
        .collect()
}

fn find_keys_for_action_groups(
    keymap: &[(KeyWithModifier, Vec<Action>)],
    action_groups: &[&[Action]],
) -> Vec<KeyWithModifier> {
    action_groups
        .iter()
        .flat_map(|actions| find_keys_for_actions(keymap, actions, true))
        .collect()
}

fn format_modifier_string(modifiers: &[KeyModifier]) -> String {
    if modifiers.is_empty() {
        String::new()
    } else {
        modifiers
            .iter()
            .map(|m| format_modifier_short(m))
            .collect::<String>()
    }
}

fn format_modifier_short(m: &KeyModifier) -> &'static str {
    match m {
        KeyModifier::Ctrl => "^",
        KeyModifier::Alt => "M-",
        KeyModifier::Shift => "S-",
        KeyModifier::Super => "Super-",
        _ => "",
    }
}

fn format_key_short(key: &KeyWithModifier) -> String {
    let modifiers: Vec<KeyModifier> = key.key_modifiers.iter().cloned().collect();
    let mods = format_modifier_string(&modifiers);
    if mods.is_empty() {
        format!("{}", key.bare_key)
    } else {
        format!("{}{}", mods, key.bare_key)
    }
}

fn format_key_display(
    key_bindings: &[KeyWithModifier],
    common_modifiers: &[KeyModifier],
) -> Vec<String> {
    key_bindings
        .iter()
        .map(|key| {
            // Always use short format (^n) instead of long format (Ctrl n)
            let unique_modifiers = key
                .key_modifiers
                .iter()
                .filter(|m| !common_modifiers.contains(m))
                .map(|m| format_modifier_short(m))
                .collect::<String>();
            if key
                .key_modifiers
                .iter()
                .all(|m| common_modifiers.contains(m))
            {
                // All modifiers are common, just show bare key
                format!("{}", key.bare_key)
            } else {
                format!("{}{}", unique_modifiers, key.bare_key)
            }
        })
        .collect()
}

fn get_key_separator(key_display: &[String]) -> &'static str {
    let key_string = key_display.join("");
    if KEY_PATTERNS_NO_SEPARATOR.contains(&&key_string[..]) {
        ""
    } else {
        "|"
    }
}

fn style_key_with_modifier(
    key_bindings: &[KeyWithModifier],
    theme: &ThemeColors,
) -> Vec<ANSIString<'static>> {
    if key_bindings.is_empty() {
        return vec![];
    }

    let mut styled_parts = vec![];

    let common_modifiers = get_common_modifiers(key_bindings.iter().collect());
    let modifier_str = format_modifier_string(&common_modifiers);
    let key_display = format_key_display(key_bindings, &common_modifiers);
    let key_separator = get_key_separator(&key_display);

    // Add leading space for readability
    styled_parts.push(Style::new().fg(theme.key_fg).on(theme.key_bg).paint(" "));

    if !modifier_str.is_empty() {
        styled_parts.push(
            Style::new()
                .fg(theme.key_fg)
                .on(theme.key_bg)
                .bold()
                .paint(modifier_str.to_string()),
        );
    }

    for (idx, key) in key_display.iter().enumerate() {
        if idx > 0 && !key_separator.is_empty() {
            styled_parts.push(
                Style::new()
                    .fg(theme.key_fg)
                    .on(theme.key_bg)
                    .paint(key_separator),
            );
        }
        styled_parts.push(
            Style::new()
                .fg(theme.key_fg)
                .on(theme.key_bg)
                .bold()
                .paint(key.clone()),
        );
    }

    // Add trailing space for readability before description arrow
    styled_parts.push(Style::new().fg(theme.key_fg).on(theme.key_bg).paint(" "));

    styled_parts
}

fn style_description(description: &str, theme: &ThemeColors) -> Vec<ANSIString<'static>> {
    // Add leading and trailing spaces for better readability
    vec![Style::new()
        .fg(theme.desc_fg)
        .on(theme.desc_bg)
        .paint(format!(" {} ", description))]
}

fn plugin_key(
    keymap: &[(KeyWithModifier, Vec<Action>)],
    plugin_name: &str,
) -> Option<KeyWithModifier> {
    keymap.iter().find_map(|(key, key_actions)| {
        if key_actions
            .iter()
            .any(|action| action.launches_plugin(plugin_name))
        {
            Some(key.clone())
        } else {
            None
        }
    })
}

fn get_select_key(keymap: &[(KeyWithModifier, Vec<Action>)]) -> Vec<KeyWithModifier> {
    let to_normal_keys = find_keys_for_actions(keymap, &[TO_NORMAL], true);
    if to_normal_keys.contains(&KeyWithModifier::new(BareKey::Enter)) {
        vec![KeyWithModifier::new(BareKey::Enter)]
    } else {
        to_normal_keys.into_iter().take(1).collect()
    }
}

fn add_hint(
    parts: &mut Vec<ANSIString<'static>>,
    keys: &[KeyWithModifier],
    description: &str,
    theme: &ThemeColors,
    prev_bg: Option<Colour>,
) -> Option<Colour> {
    if !keys.is_empty() {
        // Add powerline arrow between hints (from previous desc to current key)
        if let Some(prev) = prev_bg {
            parts.push(Style::new().fg(prev).on(theme.key_bg).paint("\u{e0b0}"));
        }

        let styled_keys = style_key_with_modifier(keys, theme);
        parts.extend(styled_keys);

        // Add arrow between key and description within this hint
        parts.push(
            Style::new()
                .fg(theme.key_bg)
                .on(theme.desc_bg)
                .paint("\u{e0b0}"),
        );

        let styled_desc = style_description(description, theme);
        parts.extend(styled_desc);

        // Return desc_bg as the next prev_bg for arrow continuity
        Some(theme.desc_bg)
    } else {
        prev_bg
    }
}

fn render_hints_for_mode(
    mode: InputMode,
    keymap: &[(KeyWithModifier, Vec<Action>)],
    theme: &ThemeColors,
    available_width: usize,
) -> Vec<ANSIString<'static>> {
    let mut parts = vec![];
    let select_keys = get_select_key(keymap);
    let mut prev_bg: Option<Colour> = None;

    match mode {
        InputMode::Normal => {
            for (action, label) in NORMAL_MODE_ACTIONS {
                let keys = find_keys_for_actions(keymap, &[action.clone()], true);
                prev_bg = add_hint(&mut parts, &keys, label, theme, prev_bg);
            }
        }
        InputMode::Pane => {
            for (actions, label) in PANE_MODE_ACTION_SEQUENCES {
                let keys = find_keys_for_actions(keymap, actions, false);
                if !keys.is_empty() {
                    prev_bg = add_hint(&mut parts, &keys, label, theme, prev_bg);
                }
            }

            let rename_keys = find_keys_for_actions(
                keymap,
                &[
                    Action::SwitchToMode(InputMode::RenamePane),
                    Action::PaneNameInput(vec![0]),
                ],
                false,
            );
            if !rename_keys.is_empty() {
                prev_bg = add_hint(&mut parts, &rename_keys, "rename", theme, prev_bg);
            }

            let focus_keys = find_keys_for_action_groups(
                keymap,
                &[
                    &[Action::MoveFocus(Direction::Left)],
                    &[Action::MoveFocus(Direction::Down)],
                    &[Action::MoveFocus(Direction::Up)],
                    &[Action::MoveFocus(Direction::Right)],
                ],
            );
            prev_bg = add_hint(&mut parts, &focus_keys, "move", theme, prev_bg);
            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Tab => {
            for (actions, label) in TAB_MODE_ACTION_SEQUENCES {
                let keys = find_keys_for_actions(keymap, actions, false);
                if !keys.is_empty() {
                    prev_bg = add_hint(&mut parts, &keys, label, theme, prev_bg);
                }
            }

            let rename_keys = find_keys_for_actions(
                keymap,
                &[
                    Action::SwitchToMode(InputMode::RenameTab),
                    Action::TabNameInput(vec![0]),
                ],
                false,
            );
            if !rename_keys.is_empty() {
                prev_bg = add_hint(&mut parts, &rename_keys, "rename", theme, prev_bg);
            }

            let focus_keys_full = find_keys_for_action_groups(
                keymap,
                &[&[Action::GoToPreviousTab], &[Action::GoToNextTab]],
            );
            let focus_keys = if focus_keys_full.contains(&KeyWithModifier::new(BareKey::Left))
                && focus_keys_full.contains(&KeyWithModifier::new(BareKey::Right))
            {
                vec![
                    KeyWithModifier::new(BareKey::Left),
                    KeyWithModifier::new(BareKey::Right),
                ]
            } else {
                focus_keys_full
            };
            prev_bg = add_hint(&mut parts, &focus_keys, "move", theme, prev_bg);
            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Resize => {
            let resize_keys = find_keys_for_action_groups(
                keymap,
                &[
                    &[Action::Resize(Resize::Increase, None)],
                    &[Action::Resize(Resize::Decrease, None)],
                ],
            );
            prev_bg = add_hint(&mut parts, &resize_keys, "resize", theme, prev_bg);

            let increase_keys = find_keys_for_action_groups(
                keymap,
                &[
                    &[Action::Resize(Resize::Increase, Some(Direction::Left))],
                    &[Action::Resize(Resize::Increase, Some(Direction::Down))],
                    &[Action::Resize(Resize::Increase, Some(Direction::Up))],
                    &[Action::Resize(Resize::Increase, Some(Direction::Right))],
                ],
            );
            prev_bg = add_hint(&mut parts, &increase_keys, "increase", theme, prev_bg);

            let decrease_keys = find_keys_for_action_groups(
                keymap,
                &[
                    &[Action::Resize(Resize::Decrease, Some(Direction::Left))],
                    &[Action::Resize(Resize::Decrease, Some(Direction::Down))],
                    &[Action::Resize(Resize::Decrease, Some(Direction::Up))],
                    &[Action::Resize(Resize::Decrease, Some(Direction::Right))],
                ],
            );
            prev_bg = add_hint(&mut parts, &decrease_keys, "decrease", theme, prev_bg);
            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Move => {
            let move_keys = find_keys_for_action_groups(
                keymap,
                &[
                    &[Action::MovePane(Some(Direction::Left))],
                    &[Action::MovePane(Some(Direction::Down))],
                    &[Action::MovePane(Some(Direction::Up))],
                    &[Action::MovePane(Some(Direction::Right))],
                ],
            );
            prev_bg = add_hint(&mut parts, &move_keys, "move", theme, prev_bg);
            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Scroll => {
            let search_keys = find_keys_for_actions(
                keymap,
                &[
                    Action::SwitchToMode(InputMode::EnterSearch),
                    Action::SearchInput(vec![0]),
                ],
                true,
            );
            prev_bg = add_hint(&mut parts, &search_keys, "search", theme, prev_bg);

            let scroll_keys =
                find_keys_for_action_groups(keymap, &[&[Action::ScrollDown], &[Action::ScrollUp]]);
            prev_bg = add_hint(&mut parts, &scroll_keys, "scroll", theme, prev_bg);

            let page_scroll_keys = find_keys_for_action_groups(
                keymap,
                &[&[Action::PageScrollDown], &[Action::PageScrollUp]],
            );
            prev_bg = add_hint(&mut parts, &page_scroll_keys, "page", theme, prev_bg);

            let half_page_scroll_keys = find_keys_for_action_groups(
                keymap,
                &[&[Action::HalfPageScrollDown], &[Action::HalfPageScrollUp]],
            );
            prev_bg = add_hint(
                &mut parts,
                &half_page_scroll_keys,
                "half page",
                theme,
                prev_bg,
            );

            let edit_keys =
                find_keys_for_actions(keymap, &[Action::EditScrollback, TO_NORMAL], false);
            if !edit_keys.is_empty() {
                prev_bg = add_hint(&mut parts, &edit_keys, "edit", theme, prev_bg);
            }
            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Search => {
            let search_keys = find_keys_for_actions(
                keymap,
                &[
                    Action::SwitchToMode(InputMode::EnterSearch),
                    Action::SearchInput(vec![0]),
                ],
                true,
            );
            prev_bg = add_hint(&mut parts, &search_keys, "search", theme, prev_bg);

            let scroll_keys =
                find_keys_for_action_groups(keymap, &[&[Action::ScrollDown], &[Action::ScrollUp]]);
            prev_bg = add_hint(&mut parts, &scroll_keys, "scroll", theme, prev_bg);

            let page_scroll_keys = find_keys_for_action_groups(
                keymap,
                &[&[Action::PageScrollDown], &[Action::PageScrollUp]],
            );
            prev_bg = add_hint(&mut parts, &page_scroll_keys, "page", theme, prev_bg);

            let half_page_scroll_keys = find_keys_for_action_groups(
                keymap,
                &[&[Action::HalfPageScrollDown], &[Action::HalfPageScrollUp]],
            );
            prev_bg = add_hint(
                &mut parts,
                &half_page_scroll_keys,
                "half page",
                theme,
                prev_bg,
            );

            let down_keys =
                find_keys_for_actions(keymap, &[Action::Search(SearchDirection::Down)], true);
            prev_bg = add_hint(&mut parts, &down_keys, "down", theme, prev_bg);

            let up_keys =
                find_keys_for_actions(keymap, &[Action::Search(SearchDirection::Up)], true);
            prev_bg = add_hint(&mut parts, &up_keys, "up", theme, prev_bg);

            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Session => {
            let detach_keys = find_keys_for_actions(keymap, &[Action::Detach], true);
            prev_bg = add_hint(&mut parts, &detach_keys, "detach", theme, prev_bg);

            if let Some(manager_key) = plugin_key(keymap, PLUGIN_SESSION_MANAGER) {
                prev_bg = add_hint(&mut parts, &[manager_key], "manager", theme, prev_bg);
            }

            if let Some(config_key) = plugin_key(keymap, PLUGIN_CONFIGURATION) {
                prev_bg = add_hint(&mut parts, &[config_key], "config", theme, prev_bg);
            }

            if let Some(plugin_key_val) = plugin_key(keymap, PLUGIN_MANAGER) {
                prev_bg = add_hint(&mut parts, &[plugin_key_val], "plugins", theme, prev_bg);
            }

            if let Some(about_key) = plugin_key(keymap, PLUGIN_ABOUT) {
                prev_bg = add_hint(&mut parts, &[about_key], "about", theme, prev_bg);
            }

            prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
        }
        InputMode::Tmux => {
            // Gateway modes only - responsive based on available width
            // Wide (>100): full labels | Medium (>60): short labels | Narrow (<60): minimal
            let is_narrow = available_width < 60;
            let is_medium = available_width < 100;

            for (action, label) in TMUX_MODE_ACTIONS {
                let keys = find_keys_for_actions(keymap, &[action.clone()], true);
                if !keys.is_empty() {
                    let display_label = label;

                    // Special handling for scroll: show both ^s and [ keys
                    if *label == "scroll" && keys.len() >= 2 {
                        // Display scroll with combined keys: "^s | [" or "^s|["
                        let separator = if is_narrow { "|" } else { " | " };
                        let combined_label = format!(
                            "{}{}{}",
                            format_key_short(&keys[0]),
                            separator,
                            format_key_short(&keys[1])
                        );
                        // Create a single key representation with combined display
                        let combined_key = KeyWithModifier::new(BareKey::Char(' '));
                        // Add arrow and styled combined key
                        if let Some(bg) = prev_bg {
                            parts.push(Style::new().fg(bg).on(theme.key_bg).paint("\u{e0b0}"));
                        }
                        parts.push(
                            Style::new()
                                .fg(theme.key_fg)
                                .on(theme.key_bg)
                                .bold()
                                .paint(format!(" {} ", combined_label)),
                        );
                        parts.push(
                            Style::new()
                                .fg(theme.key_bg)
                                .on(theme.desc_bg)
                                .paint("\u{e0b0}"),
                        );
                        parts.push(
                            Style::new()
                                .fg(theme.desc_fg)
                                .on(theme.desc_bg)
                                .paint(format!(" {}", display_label)),
                        );
                        prev_bg = Some(theme.desc_bg);
                    } else {
                        prev_bg = add_hint(&mut parts, &keys, display_label, theme, prev_bg);
                    }
                }
            }

            // Only show "select" hint if we have space
            if !is_narrow {
                prev_bg = add_hint(&mut parts, &select_keys, "select", theme, prev_bg);
            }
        }
        _ => {
            let keys =
                find_keys_for_actions(keymap, &[Action::SwitchToMode(InputMode::Normal)], true);
            prev_bg = add_hint(&mut parts, &keys, "normal", theme, prev_bg);
        }
    }

    parts
}

fn get_keymap_for_mode(mode_info: &ModeInfo) -> Vec<(KeyWithModifier, Vec<Action>)> {
    match mode_info.mode {
        InputMode::Normal => mode_info.get_keybinds_for_mode(InputMode::Normal),
        InputMode::Pane => mode_info.get_keybinds_for_mode(InputMode::Pane),
        InputMode::Tab => mode_info.get_keybinds_for_mode(InputMode::Tab),
        InputMode::Resize => mode_info.get_keybinds_for_mode(InputMode::Resize),
        InputMode::Move => mode_info.get_keybinds_for_mode(InputMode::Move),
        InputMode::Scroll => mode_info.get_keybinds_for_mode(InputMode::Scroll),
        InputMode::Search => mode_info.get_keybinds_for_mode(InputMode::Search),
        InputMode::Session => mode_info.get_keybinds_for_mode(InputMode::Session),
        InputMode::Tmux => mode_info.get_keybinds_for_mode(InputMode::Tmux),
        _ => mode_info.get_mode_keybinds(),
    }
}
