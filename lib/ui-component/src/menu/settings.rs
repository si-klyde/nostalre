//! Settings: the option pages in the menu's frame, pages down the left and their options beside them. A change
//! applies at once, sent as the classic option windows send theirs (`docs/nostalre-menu.md`, "Settings").
use super::frame::{self, COLUMN_GAP, HANDHELD, Hint, Key, Keys, ROW_H, ROW_TEXT};
use super::{ROW_BASE_ID, SETTINGS_KEY_BASE_ID, SETTINGS_PAGE_BASE_ID, SETTINGS_RESET_ID};
use crate::game::graphic_options::{UI_SCALE_LABELS, UI_SCALE_OPTIONS, ui_scale_index};
use crate::game::shortcut_list_window::{LINE_MAX, SLOT_COUNT};
use crate::hud::style;
use nostalre_game::display::{GraphicsSettings, HudLayout, SoundSettings};
use nostalre_game::event::GameEvent;
use nostalre_game::keybinding::{HotkeyAction, KeyBindings, KeyChord};
use nostalre_ui::frame::{UiFrame, WidgetId};
use nostalre_ui::rect::Rect;
use nostalre_ui::text_input::TextInput;

const PAGES_W: f32 = 200.0;
const CHOICE_W: f32 = 110.0;
const SLIDER_W: f32 = 160.0;
const SECTION_GAP: f32 = 12.0;
const VOLUME_STEP: u32 = 5;
const LINE_LABEL_W: f32 = 90.0;
/// The chat lines' keys, Alt+1 to Alt+9 then Alt+0
const LINE_LABELS: [&str; SLOT_COUNT] = [
    "Alt+1", "Alt+2", "Alt+3", "Alt+4", "Alt+5", "Alt+6", "Alt+7", "Alt+8", "Alt+9", "Alt+0",
];
const PAD_NOTE: &str =
    "Controller buttons are fixed: A, B, X, Y, L1 and R1 as the menu's hints show.";

/// What the pages show and change, as the client's config holds it
#[derive(Debug, Clone, Default)]
pub struct SettingsValues {
    pub graphics: GraphicsSettings,
    pub sound: SoundSettings,
    pub hud: HudLayout,
    pub keys: KeyBindings,
    /// Alt+1 to Alt+0
    pub chat_lines: Vec<String>,
    /// X (aim assist) also attacks what it locks
    pub aim_attack: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opt {
    Fullscreen,
    Scale,
    Hud,
    FilterWorld,
    FilterEffects,
    FilterSprites,
    Upscale,
    Fog,
    SkillEffects,
    Aura,
    NamePlayer,
    NameMonster,
    NameNpc,
    OtherDamage,
    OtherCast,
    Clearer,
    RefuseTrade,
    RefuseParty,
    SnapMonster,
    SnapSkill,
    SnapItem,
    AimAttack,
    Music,
    Effects,
    Stereo,
    Background,
}

/// Which event a change sends
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Graphics,
    Hud,
    Sound,
    Aim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    /// Left: off, the choice before, quieter
    Prev,
    /// Right: on, the choice after, louder
    Next,
    /// Enter: the other way, the next choice round
    Flip,
}

impl Opt {
    fn label(self) -> &'static str {
        match self {
            Self::Fullscreen => "Full screen",
            Self::Scale => "Interface scale",
            Self::Hud => "HUD",
            Self::FilterWorld => "Map textures",
            Self::FilterEffects => "Effect textures",
            Self::FilterSprites => "Sprite textures",
            Self::Upscale => "Sprite upscale",
            Self::Fog => "Fog",
            Self::SkillEffects => "Skill effects",
            Self::Aura => "Level 99 aura",
            Self::NamePlayer => "Players",
            Self::NameMonster => "Monsters",
            Self::NameNpc => "NPCs",
            Self::OtherDamage => "Their damage numbers",
            Self::OtherCast => "Their cast bars",
            Self::Clearer => "Bold name plates",
            Self::RefuseTrade => "Trade requests",
            Self::RefuseParty => "Party invites",
            Self::SnapMonster => "To monsters",
            Self::SnapSkill => "To monsters, skill ready",
            Self::SnapItem => "To items",
            Self::AimAttack => "X attacks",
            Self::Music => "Music",
            Self::Effects => "Effects",
            Self::Stereo => "Stereo",
            Self::Background => "Play in background",
        }
    }

    fn help(self) -> &'static str {
        match self {
            Self::Fullscreen => "Fill the screen. Off: a window you can resize.",
            Self::Scale => "How large the HUD, the menu and text draw.",
            Self::Hud => {
                "Frameless: nostalRE's HUD and this menu. Classic: kRO's windows (closes this menu)."
            }
            Self::FilterWorld => "Smooth ground and model textures. Off: sharp pixels.",
            Self::FilterEffects => "Smooth skill and effect textures.",
            Self::FilterSprites => "Smooth characters and monsters.",
            Self::Upscale => "Sharpen smoothed sprites as they are scaled up.",
            Self::Fog => "The map's distance fog.",
            Self::SkillEffects => "Draw skill effects.",
            Self::Aura => "The glow around max-level characters.",
            Self::NamePlayer => "Names over other players.",
            Self::NameMonster => "Names over monsters.",
            Self::NameNpc => "Names over NPCs.",
            Self::OtherDamage => "Damage dealt by other players.",
            Self::OtherCast => "Cast bars over other players.",
            Self::Clearer => "Names over the world in bold.",
            Self::RefuseTrade => "Turn down trade requests without asking.",
            Self::RefuseParty => "Turn down party invites without asking.",
            Self::SnapMonster => "The cursor jumps to a monster near it.",
            Self::SnapSkill => "The same, only while a skill waits for a target.",
            Self::SnapItem => "The cursor jumps to an item on the ground.",
            Self::AimAttack => {
                "X locks a monster and attacks it. Off: X only aims, again the next; your skills cast on the lock."
            }
            Self::Music => "Background music.",
            Self::Effects => "Skills, hits and the interface.",
            Self::Stereo => "Sounds pan with where they happen.",
            Self::Background => "Keep sound on while nostalRE is not in front.",
        }
    }

    /// What Enter does to it; a volume moves with left and right only
    fn verb(self) -> Option<&'static str> {
        match self {
            Self::Scale | Self::Hud => Some("change"),
            Self::Music | Self::Effects => None,
            _ => Some("toggle"),
        }
    }

    fn group(self) -> Group {
        match self {
            Self::Hud => Group::Hud,
            Self::AimAttack => Group::Aim,
            Self::Music | Self::Effects | Self::Stereo | Self::Background => Group::Sound,
            _ => Group::Graphics,
        }
    }

    /// A handheld has no window to leave
    fn shown(self) -> bool {
        !(HANDHELD && self == Self::Fullscreen)
    }

    /// An on/off option's flag; for a name, the flag hides it
    fn flag(self, v: &mut SettingsValues) -> Option<&mut bool> {
        let (g, s) = (&mut v.graphics, &mut v.sound);
        Some(match self {
            Self::Fullscreen => &mut g.fullscreen,
            Self::FilterWorld => &mut g.filter_world,
            Self::FilterEffects => &mut g.filter_effects,
            Self::FilterSprites => &mut g.filter_sprites,
            Self::Upscale => &mut g.sprite_upscale,
            Self::Fog => &mut g.fog,
            Self::SkillEffects => &mut g.show_skill_effects,
            Self::Aura => &mut g.display.show_level_aura,
            Self::NamePlayer => &mut g.display.hide_name_player,
            Self::NameMonster => &mut g.display.hide_name_monster,
            Self::NameNpc => &mut g.display.hide_name_npc,
            Self::OtherDamage => &mut g.display.show_other_damage,
            Self::OtherCast => &mut g.display.show_other_cast_bars,
            Self::Clearer => &mut g.accessibility,
            Self::RefuseTrade => &mut g.refuse_trade,
            Self::RefuseParty => &mut g.refuse_party_invite,
            Self::SnapMonster => &mut g.snap.monster_no_skill,
            Self::SnapSkill => &mut g.snap.monster_skill,
            Self::SnapItem => &mut g.snap.item,
            Self::AimAttack => &mut v.aim_attack,
            Self::Stereo => &mut s.stereo,
            Self::Background => &mut s.play_when_unfocused,
            Self::Scale | Self::Hud | Self::Music | Self::Effects => return None,
        })
    }

    fn hides(self) -> bool {
        matches!(self, Self::NamePlayer | Self::NameMonster | Self::NameNpc)
    }

    /// A volume and whether it plays
    fn volume(self, v: &mut SettingsValues) -> Option<(&mut f32, &mut bool)> {
        let s = &mut v.sound;
        match self {
            Self::Music => Some((&mut s.bgm_volume, &mut s.bgm_enabled)),
            Self::Effects => Some((&mut s.sfx_volume, &mut s.sfx_enabled)),
            _ => None,
        }
    }

    fn shown_value(self, v: &mut SettingsValues) -> Shown {
        if let Some(flag) = self.flag(v) {
            return Shown::Toggle(*flag != self.hides());
        }
        if let Some((volume, on)) = self.volume(v) {
            return Shown::Slider(percent(*volume, *on));
        }
        Shown::Choice(match self {
            Self::Hud => hud_label(v.hud),
            _ => UI_SCALE_LABELS[ui_scale_index(v.graphics.ui_scale)],
        })
    }

    /// True when the value changed
    fn change(self, v: &mut SettingsValues, how: Change) -> bool {
        let hides = self.hides();
        if let Some(flag) = self.flag(v) {
            let old = *flag;
            *flag = match how {
                Change::Prev => hides,
                Change::Next => !hides,
                Change::Flip => !old,
            };
            return *flag != old;
        }
        if let Some((volume, on)) = self.volume(v) {
            return step_volume(volume, on, how);
        }
        match self {
            Self::Hud => {
                let old = v.hud;
                v.hud = match (how, old) {
                    (Change::Prev, _) | (Change::Flip, HudLayout::Classic) => HudLayout::Modern,
                    (Change::Next, _) | (Change::Flip, HudLayout::Modern) => HudLayout::Classic,
                };
                v.hud != old
            }
            _ => {
                let old = ui_scale_index(v.graphics.ui_scale);
                let last = UI_SCALE_OPTIONS.len() - 1;
                let new = match how {
                    Change::Prev => old.saturating_sub(1),
                    Change::Next => (old + 1).min(last),
                    Change::Flip => (old + 1) % UI_SCALE_OPTIONS.len(),
                };
                let scale = UI_SCALE_OPTIONS[new] as f32;
                std::mem::replace(&mut v.graphics.ui_scale, scale) != scale
            }
        }
    }

    /// Back to `defaults`' value; true when that changed it
    fn reset(self, v: &mut SettingsValues, defaults: &mut SettingsValues) -> bool {
        if let (Some(flag), Some(&mut default)) = (self.flag(v), self.flag(defaults)) {
            return std::mem::replace(flag, default) != default;
        }
        if let (Some((volume, on)), Some((dv, don))) = (self.volume(v), self.volume(defaults)) {
            let changed = *volume != *dv || *on != *don;
            (*volume, *on) = (*dv, *don);
            return changed;
        }
        match self {
            Self::Hud => std::mem::replace(&mut v.hud, defaults.hud) != defaults.hud,
            _ => {
                let scale = defaults.graphics.ui_scale;
                std::mem::replace(&mut v.graphics.ui_scale, scale) != scale
            }
        }
    }
}

/// A volume as the slider shows it: 0 to 100, 0 while it does not play
fn percent(volume: f32, on: bool) -> u32 {
    if on {
        (volume.clamp(0.0, 1.0) * 100.0).round() as u32
    } else {
        0
    }
}

/// The slider at `percent`: at 0 the sound is off, above it on
fn set_volume(volume: &mut f32, on: &mut bool, percent: u32) {
    *volume = percent as f32 / 100.0;
    *on = percent > 0;
}

/// Left or right on a volume, onto the steps (29 goes to 25 or 30). One muted elsewhere comes back at its own
/// level on the first step up; true when it changed
fn step_volume(volume: &mut f32, on: &mut bool, how: Change) -> bool {
    let old = percent(*volume, *on);
    if how == Change::Next && !*on && *volume > 0.0 {
        *on = true;
        return true;
    }
    let new = match how {
        Change::Prev => old.saturating_sub(1) / VOLUME_STEP * VOLUME_STEP,
        Change::Next => ((old / VOLUME_STEP + 1) * VOLUME_STEP).min(100),
        Change::Flip => return false,
    };
    set_volume(volume, on, new);
    new != old
}

fn hud_label(hud: HudLayout) -> &'static str {
    match hud {
        HudLayout::Modern => "Frameless",
        HudLayout::Classic => "Classic",
    }
}

enum Shown {
    Toggle(bool),
    Choice(&'static str),
    Slider(u32),
}

enum Entry {
    Section(&'static str),
    Opt(Opt),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Page {
    #[default]
    Display,
    Graphics,
    OnScreen,
    Gameplay,
    Sound,
    Keys,
    ChatLines,
}

const PAGES: [Page; 7] = [
    Page::Display,
    Page::Graphics,
    Page::OnScreen,
    Page::Gameplay,
    Page::Sound,
    Page::Keys,
    Page::ChatLines,
];

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Graphics => "Graphics",
            Self::OnScreen => "On Screen",
            Self::Gameplay => "Gameplay",
            Self::Sound => "Sound",
            Self::Keys => "Keys",
            Self::ChatLines => "Chat Lines",
        }
    }

    fn entries(self) -> &'static [Entry] {
        use Entry::{Opt as O, Section as S};
        match self {
            Self::Display => &[S("Screen"), O(Opt::Fullscreen), O(Opt::Scale), O(Opt::Hud)],
            Self::Graphics => &[
                S("Smoothing"),
                O(Opt::FilterWorld),
                O(Opt::FilterEffects),
                O(Opt::FilterSprites),
                O(Opt::Upscale),
                S("World"),
                O(Opt::Fog),
                O(Opt::SkillEffects),
                O(Opt::Aura),
            ],
            Self::OnScreen => &[
                S("Names"),
                O(Opt::NamePlayer),
                O(Opt::NameMonster),
                O(Opt::NameNpc),
                O(Opt::Clearer),
                S("Other players"),
                O(Opt::OtherDamage),
                O(Opt::OtherCast),
            ],
            Self::Gameplay => &[
                S("Auto-refuse"),
                O(Opt::RefuseTrade),
                O(Opt::RefuseParty),
                S("Cursor snap"),
                O(Opt::SnapMonster),
                O(Opt::SnapSkill),
                O(Opt::SnapItem),
                S("Aim assist"),
                O(Opt::AimAttack),
            ],
            Self::Sound => &[
                S("Volume"),
                O(Opt::Music),
                O(Opt::Effects),
                S("Output"),
                O(Opt::Stereo),
                O(Opt::Background),
            ],
            Self::Keys | Self::ChatLines => &[],
        }
    }

    /// The options a player can pick, in order
    fn opts(self) -> impl Iterator<Item = Opt> {
        self.entries().iter().filter_map(|e| match e {
            Entry::Opt(o) if o.shown() => Some(*o),
            _ => None,
        })
    }
}

/// An action as the menu names it: its commands' names, not the classic windows'
fn key_name(action: HotkeyAction) -> &'static str {
    match action {
        HotkeyAction::ToggleInventory => "Items",
        HotkeyAction::ToggleShortcutList => "Chat Lines",
        HotkeyAction::ToggleEmotion => "Emotions",
        HotkeyAction::ToggleQuest => "Quests",
        _ => action.label(),
    }
}

/// A binding as the menu writes it: `Alt+E`, a dash for none
fn chord_label(chord: Option<&KeyChord>) -> String {
    chord.map_or_else(|| "—".to_string(), KeyChord::short_display)
}

struct Loaded {
    now: SettingsValues,
    defaults: SettingsValues,
    /// Each action's key as the Keys page writes it, in `HotkeyAction::ALL`'s order
    key_labels: Vec<String>,
}

impl Loaded {
    fn new(mut now: SettingsValues, defaults: SettingsValues) -> Self {
        now.chat_lines.resize(SLOT_COUNT, String::new());
        let mut loaded = Self {
            now,
            defaults,
            key_labels: Vec::new(),
        };
        loaded.label_keys();
        loaded
    }

    fn label_keys(&mut self) {
        let keys = &self.now.keys;
        self.key_labels = HotkeyAction::ALL
            .iter()
            .map(|a| chord_label(keys.get(*a)))
            .collect();
    }
}

#[derive(Default)]
pub(super) struct Settings {
    page: Page,
    picked: usize,
    /// Read from the client each time the screen opens
    values: Option<Box<Loaded>>,
    /// The action whose new key the next press is
    capturing: Option<HotkeyAction>,
    /// The picked chat line, being typed
    editing: Option<TextInput>,
    /// What the last key press did, said in the help line until the next move
    note: Option<String>,
}

impl Settings {
    /// The screen is open without the client's values
    pub fn needs_values(&self) -> bool {
        self.values.is_none()
    }

    pub fn set_values(&mut self, now: SettingsValues, defaults: SettingsValues) {
        self.values = Some(Box::new(Loaded::new(now, defaults)));
    }

    /// The screen closed: values are read again next time, a key or line being changed is dropped
    pub fn leave(&mut self) {
        self.values = None;
        self.capturing = None;
        self.editing = None;
        self.note = None;
    }

    /// Escape: out of a key being set or a line being typed; false when there was none
    pub fn back(&mut self) -> bool {
        self.note = None;
        self.capturing.take().is_some() || self.editing.take().is_some()
    }

    pub fn cancel_capture(&mut self) {
        self.capturing = None;
        self.note = None;
    }

    pub fn is_capturing(&self) -> bool {
        self.capturing.is_some()
    }

    /// A chat line is being typed: the keys are text, not the game's
    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// The key pressed for the action waiting for one. One another action had is swapped: that action gets
    /// the old key. A key the game keeps for itself is refused and the page goes on waiting. The client
    /// takes the press away from the rest of the frame (`UiContext::forget_key_press`)
    pub fn capture(&mut self, chord: KeyChord) -> Option<GameEvent> {
        let action = self.capturing?;
        let loaded = self.values.as_mut()?;
        let keys = &mut loaded.now.keys;
        if chord.is_reserved() {
            self.note = Some(format!(
                "{} is the game's own key. Press another.",
                chord.short_display()
            ));
            return None;
        }
        let old = keys.get(action).cloned();
        let taken = keys.conflict(&chord, action);
        self.note = taken.map(|other| match &old {
            Some(old) => format!(
                "{} took {}; {} now has {}.",
                key_name(action),
                chord.short_display(),
                key_name(other),
                old.short_display()
            ),
            None => format!("{} took {}.", key_name(action), chord.short_display()),
        });
        if let (Some(other), Some(old)) = (taken, old) {
            keys.set(other, old);
        }
        keys.set(action, chord);
        let event = GameEvent::KeyBindingsChanged(Box::new(keys.clone()));
        loaded.label_keys();
        self.capturing = None;
        Some(event)
    }

    pub fn footer(&self) -> (String, Vec<Hint>) {
        let action = |label| Hint::tap("Enter", label, Key::Enter);
        if let Some(waiting) = self.capturing {
            let keeps = self
                .values
                .as_ref()
                .map(|v| chord_label(v.now.keys.get(waiting)));
            let help = self.note.clone().unwrap_or_else(|| {
                format!(
                    "Press the new key for {}. Escape keeps {}.",
                    key_name(waiting),
                    keeps.unwrap_or_default()
                )
            });
            return (help, vec![Hint::CANCEL]);
        }
        if self.editing.is_some() {
            let help = format!("Type {}'s line.", LINE_LABELS[self.picked]);
            return (help, vec![action("keep"), Hint::CANCEL]);
        }
        let (help, verb) = match self.page {
            Page::Keys => {
                let help = HotkeyAction::ALL
                    .get(self.picked)
                    .map(|a| format!("{}: Enter, then press its new key.", key_name(*a)))
                    .unwrap_or_default();
                (help, Some("change"))
            }
            Page::ChatLines => (
                format!("Sent to chat with {}.", LINE_LABELS[self.picked]),
                Some("edit"),
            ),
            page => match page.opts().nth(self.picked) {
                Some(opt) => (opt.help().to_string(), opt.verb()),
                None => (String::new(), None),
            },
        };
        let arrows = match self.page {
            Page::Keys | Page::ChatLines => Hint::MOVE,
            _ => Hint::MOVE_CHANGE,
        };
        let mut hints = vec![arrows, Hint::PAGES];
        hints.extend(verb.map(action));
        hints.extend([Hint::tap("Del", "default", Key::Remove), Hint::BACK]);
        (self.note.clone().unwrap_or(help), hints)
    }

    pub fn build(
        &mut self,
        ui: &mut UiFrame,
        area: Rect,
        keys: &Keys,
        events: &mut Vec<GameEvent>,
    ) {
        let Some(mut values) = self.values.take() else {
            return;
        };
        // a key being set or a line being typed has the keys to itself: only a tap turns the page
        let busy = self.capturing.is_some() || self.editing.is_some();
        let free = if busy { Keys::default() } else { *keys };
        let pages_w = PAGES_W.min(area.w * 0.3).floor();
        let turned = self.pages(ui, area, pages_w, &free);
        let reset_all = self.reset_row(ui, area, pages_w);
        if let Some(page) = turned {
            // a line being typed is kept, as Enter keeps it
            self.commit_line(&mut values.now, events);
            self.turn_to(page);
        }
        let x = area.x + pages_w + COLUMN_GAP;
        let opts = Rect::new(x, area.y, (area.x + area.w - x).max(0.0), area.h);
        let v = &mut *values;
        match self.page {
            Page::Keys => self.keys_page(ui, opts, &free, reset_all, v, events),
            Page::ChatLines => self.lines_page(ui, opts, &free, keys.enter, reset_all, v, events),
            page => self.options_page(ui, opts, page, &free, reset_all, v, events),
        }
        self.values = Some(values);
    }

    fn turn_to(&mut self, page: Page) {
        if page != self.page {
            self.page = page;
            self.picked = 0;
        }
        self.capturing = None;
        self.editing = None;
        self.note = None;
    }

    /// The page list, the open one in bold; returns the one turned to by a tap or the page keys
    fn pages(&mut self, ui: &mut UiFrame, area: Rect, w: f32, keys: &Keys) -> Option<Page> {
        let at = PAGES.iter().position(|&p| p == self.page).unwrap_or(0);
        let mut turned = frame::switch_tab(keys, at, PAGES.len(), None);
        for (i, page) in PAGES.iter().enumerate() {
            let rect = Rect::new(area.x, area.y + i as f32 * ROW_H, w, ROW_H);
            let resp = ui.interact(WidgetId(SETTINGS_PAGE_BASE_ID + i as u32), rect);
            let baseline = frame::row_baseline(ui, rect);
            if i == at {
                let size = (ROW_H * 0.32).floor();
                let y = rect.y + ((ROW_H - size) / 2.0).floor();
                style::caret(ui, rect.x + 6.0, y, size, style::ACCENT);
                style::text_bold(ui, rect.x + ROW_TEXT, baseline, page.label(), style::TEXT);
            } else {
                let color = if resp.hovered() {
                    style::TEXT
                } else {
                    style::DIM
                };
                style::text(ui, rect.x + ROW_TEXT, baseline, page.label(), color);
            }
            if resp.clicked() {
                turned = i;
            }
        }
        (turned != at).then_some(PAGES[turned])
    }

    /// "Reset page" under the page list; true when tapped
    fn reset_row(&mut self, ui: &mut UiFrame, area: Rect, w: f32) -> bool {
        let y = area.y + PAGES.len() as f32 * ROW_H + SECTION_GAP;
        style::rule(ui, area.x, y, w);
        let rect = Rect::new(area.x, y + SECTION_GAP, w, ROW_H);
        let resp = ui.interact(SETTINGS_RESET_ID, rect);
        let color = if resp.hovered() {
            style::TEXT
        } else {
            style::DIM
        };
        let baseline = frame::row_baseline(ui, rect);
        style::text(ui, rect.x + ROW_TEXT, baseline, "Reset page", color);
        resp.clicked()
    }

    #[allow(clippy::too_many_arguments)]
    fn options_page(
        &mut self,
        ui: &mut UiFrame,
        area: Rect,
        page: Page,
        keys: &Keys,
        reset_all: bool,
        loaded: &mut Loaded,
        events: &mut Vec<GameEvent>,
    ) {
        let count = page.opts().count();
        let before = self.picked;
        self.picked = frame::step(keys, self.picked, count);
        if self.picked != before {
            self.note = None;
        }
        let mut changed: Vec<Opt> = Vec::new();
        let Loaded { now, defaults, .. } = loaded;
        if reset_all {
            changed.extend(page.opts().filter(|o| o.reset(now, defaults)));
        }
        if let Some(opt) = page.opts().nth(self.picked) {
            let how = if keys.left {
                Some(Change::Prev)
            } else if keys.right {
                Some(Change::Next)
            } else if keys.enter {
                Some(Change::Flip)
            } else {
                None
            };
            let stepped = how.is_some_and(|how| opt.change(now, how));
            if stepped || (keys.remove && opt.reset(now, defaults)) {
                changed.push(opt);
            }
        }

        let mut y = area.y;
        let mut index = 0;
        for (n, entry) in page.entries().iter().enumerate() {
            match entry {
                Entry::Section(heading) => {
                    if n > 0 {
                        y += SECTION_GAP;
                    }
                    y = frame::section(ui, area.x, y, area.w, heading);
                }
                Entry::Opt(opt) if opt.shown() => {
                    let rect = Rect::new(area.x, y, area.w, ROW_H);
                    let id = WidgetId(ROW_BASE_ID + index as u32);
                    let resp = frame::row(ui, id, rect, index == self.picked);
                    let baseline = frame::row_baseline(ui, rect);
                    style::text(ui, rect.x + ROW_TEXT, baseline, opt.label(), style::TEXT);
                    let tap = value(ui, rect, opt.shown_value(now));
                    if resp.clicked() {
                        self.picked = index;
                        self.note = None;
                        let tap_changed = match tap {
                            Tap::Change(how) => opt.change(now, how),
                            Tap::Volume(to) => opt.volume(now).is_some_and(|(volume, on)| {
                                let old = percent(*volume, *on);
                                set_volume(volume, on, to);
                                old != to
                            }),
                            Tap::None => false,
                        };
                        if tap_changed {
                            changed.push(*opt);
                        }
                    }
                    y += ROW_H;
                    index += 1;
                }
                Entry::Opt(_) => {}
            }
        }

        let sent = |g: Group| changed.iter().any(|o| o.group() == g);
        if sent(Group::Graphics) {
            events.push(GameEvent::GraphicsSettingsChanged {
                settings: now.graphics.clone(),
                persist: true,
            });
        }
        if sent(Group::Sound) {
            events.push(GameEvent::SoundSettingsChanged {
                settings: now.sound,
                persist: true,
            });
        }
        if sent(Group::Hud) {
            events.push(GameEvent::HudLayoutChanged { layout: now.hud });
        }
        if sent(Group::Aim) {
            events.push(GameEvent::AimSettingsChanged {
                x_attacks: now.aim_attack,
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn keys_page(
        &mut self,
        ui: &mut UiFrame,
        area: Rect,
        keys: &Keys,
        reset_all: bool,
        loaded: &mut Loaded,
        events: &mut Vec<GameEvent>,
    ) {
        let all = &HotkeyAction::ALL;
        let per_column = all.len().div_ceil(2);
        let before = self.picked;
        self.picked = frame::step(keys, self.picked, all.len());
        if keys.left || keys.right {
            self.picked = if self.picked < per_column {
                (self.picked + per_column).min(all.len() - 1)
            } else {
                self.picked - per_column
            };
        }
        if self.picked != before {
            self.note = None;
        }
        let Loaded { now, defaults, .. } = &mut *loaded;
        let mut changed = false;
        if reset_all {
            changed = now.keys != defaults.keys;
            now.keys = defaults.keys.clone();
        }
        if keys.enter {
            self.capturing = Some(all[self.picked]);
            self.note = None;
        }
        if keys.remove {
            changed |= reset_key(all[self.picked], now, defaults);
        }
        if changed {
            events.push(GameEvent::KeyBindingsChanged(Box::new(now.keys.clone())));
            loaded.label_keys();
        }

        let y = frame::section(ui, area.x, area.y, area.w, "Keyboard");
        let col_w = ((area.w - COLUMN_GAP) / 2.0).floor();
        for (i, action) in all.iter().enumerate() {
            let (col, row) = (i / per_column, i % per_column);
            let x = area.x + col as f32 * (col_w + COLUMN_GAP);
            let rect = Rect::new(x, y + row as f32 * ROW_H, col_w, ROW_H);
            let id = WidgetId(SETTINGS_KEY_BASE_ID + i as u32);
            let resp = frame::row(ui, id, rect, i == self.picked);
            let baseline = frame::row_baseline(ui, rect);
            style::text(
                ui,
                rect.x + ROW_TEXT,
                baseline,
                key_name(*action),
                style::TEXT,
            );
            let right = rect.x + rect.w - 8.0;
            if self.capturing == Some(*action) {
                style::text_right(ui, right, baseline, "press a key", style::ACCENT);
            } else {
                let label = &loaded.key_labels[i];
                let w = style::key_cap_width(ui, label);
                let top = rect.y + ((ROW_H - style::line(ui)) / 2.0).floor();
                style::key_cap(ui, right - w, top, label);
            }
            if resp.clicked() {
                self.picked = i;
                self.capturing = Some(*action);
                self.note = None;
            }
        }
        let note_y = y + per_column as f32 * ROW_H + SECTION_GAP;
        let pad = style::fit(ui, PAD_NOTE, area.w);
        style::text(ui, area.x, style::baseline(ui, note_y), &pad, style::DIM);
    }

    /// The line being typed into the picked slot, sent; nothing when none is
    fn commit_line(&mut self, now: &mut SettingsValues, events: &mut Vec<GameEvent>) {
        if let Some(input) = self.editing.take()
            && now.chat_lines[self.picked] != input.text
        {
            now.chat_lines[self.picked] = input.text;
            events.push(GameEvent::ShortcutBindingsChanged(now.chat_lines.clone()));
        }
    }

    /// `commit_tap`: the footer's "keep" was tapped (the menu's keys, which the page does not get while typing)
    #[allow(clippy::too_many_arguments)]
    fn lines_page(
        &mut self,
        ui: &mut UiFrame,
        area: Rect,
        keys: &Keys,
        commit_tap: bool,
        reset_all: bool,
        loaded: &mut Loaded,
        events: &mut Vec<GameEvent>,
    ) {
        let Loaded { now, defaults, .. } = loaded;
        let mut changed = false;
        if reset_all {
            self.editing = None;
            changed = now.chat_lines != defaults.chat_lines;
            now.chat_lines.clone_from(&defaults.chat_lines);
            now.chat_lines.resize(SLOT_COUNT, String::new());
        }
        // typing: the menu's keys are held back, the field reads the raw ones
        if let Some(input) = &mut self.editing {
            input.process_keys(ui.ctx);
            if ui.ctx.key_enter || commit_tap {
                self.commit_line(now, events);
            }
        } else {
            self.picked = frame::step(keys, self.picked, SLOT_COUNT);
            if keys.enter {
                self.edit(&now.chat_lines[self.picked]);
            }
            if keys.remove {
                let default = defaults.chat_lines.get(self.picked).cloned();
                let default = default.unwrap_or_default();
                changed |= std::mem::replace(&mut now.chat_lines[self.picked], default)
                    != now.chat_lines[self.picked];
            }
        }

        let y = frame::section(ui, area.x, area.y, area.w, "Alt + 1 to 0");
        let mut tapped = None;
        for (i, label) in LINE_LABELS.iter().enumerate() {
            let rect = Rect::new(area.x, y + i as f32 * ROW_H, area.w, ROW_H);
            let resp = frame::row(ui, WidgetId(ROW_BASE_ID + i as u32), rect, i == self.picked);
            let baseline = frame::row_baseline(ui, rect);
            style::text(ui, rect.x + ROW_TEXT, baseline, label, style::DIM);
            let tx = rect.x + LINE_LABEL_W;
            let max_w = (rect.x + rect.w - 8.0 - tx).max(0.0);
            match self.editing.as_ref().filter(|_| i == self.picked) {
                Some(input) => {
                    let text = input.display_text();
                    style::text(ui, tx, baseline, &text, style::TEXT);
                    let before = &text[..input.display_cursor_offset().min(text.len())];
                    let cx = tx + style::measure(ui, before);
                    let line = style::line(ui);
                    let top = rect.y + ((ROW_H - line) / 2.0).floor();
                    style::fill(ui, Rect::new(cx, top, 1.0, line), style::ACCENT);
                    style::underline_accent(ui, Rect::new(tx, top, max_w, line + 4.0));
                }
                None if now.chat_lines[i].is_empty() => {
                    style::text(ui, tx, baseline, "—", style::DIM);
                }
                None => {
                    let text = style::fit(ui, &now.chat_lines[i], max_w);
                    style::text(ui, tx, baseline, &text, style::TEXT);
                }
            }
            if resp.clicked() && self.editing.is_none() {
                tapped = Some(i);
            }
        }
        if let Some(i) = tapped {
            self.picked = i;
            self.edit(&now.chat_lines[i]);
        }
        if changed {
            events.push(GameEvent::ShortcutBindingsChanged(now.chat_lines.clone()));
        }
    }

    fn edit(&mut self, line: &str) {
        let mut input = TextInput::new(LINE_MAX, false);
        input.text = line.to_string();
        input.move_cursor_to_end();
        self.editing = Some(input);
        self.note = None;
    }
}

/// One action back to its default key, swapped with whichever action had it
fn reset_key(action: HotkeyAction, now: &mut SettingsValues, defaults: &SettingsValues) -> bool {
    let Some(default) = defaults.keys.get(action).cloned() else {
        return false;
    };
    if now.keys.get(action) == Some(&default) {
        return false;
    }
    let holder = now.keys.conflict(&default, action);
    if let (Some(other), Some(old)) = (holder, now.keys.get(action).cloned()) {
        now.keys.set(other, old);
    }
    now.keys.set(action, default);
    true
}

/// What a tap on an option's row does, by where it landed
enum Tap {
    Change(Change),
    /// A volume, from where on the slider
    Volume(u32),
    None,
}

/// An option's value at the right of its row; returns what a tap there would do: a toggle flips, a choice goes
/// back on its left half and on on its right, a slider takes the tap's place
fn value(ui: &mut UiFrame, rect: Rect, shown: Shown) -> Tap {
    let right = rect.x + rect.w - 8.0;
    let baseline = frame::row_baseline(ui, rect);
    let line = style::line(ui);
    let top = rect.y + ((ROW_H - line) / 2.0).floor();
    let mouse_x = ui.ctx.mouse_x;
    match shown {
        Shown::Toggle(on) => {
            let word_w = style::measure(ui, "Off");
            let word = if on { "On" } else { "Off" };
            style::text(ui, right - word_w, baseline, word, style::DIM);
            let h = (line - 2.0).floor();
            let x = right - word_w - 10.0 - style::switch_width(h);
            style::switch(ui, x, top + 1.0, h, on);
            Tap::Change(Change::Flip)
        }
        Shown::Choice(label) => {
            let size = (line * 0.5).floor();
            let y = top + ((line - size) / 2.0).floor();
            let left = right - CHOICE_W;
            style::caret_left(ui, left, y, size, style::DIM);
            style::caret(ui, right - (size * 0.8).round(), y, size, style::DIM);
            style::text_centered(ui, left + CHOICE_W / 2.0, baseline, label, style::TEXT);
            let earlier = mouse_x >= left && mouse_x < left + CHOICE_W / 2.0;
            Tap::Change(if earlier { Change::Prev } else { Change::Next })
        }
        Shown::Slider(percent) => {
            let number_w = style::measure(ui, "100");
            style::text_right(ui, right, baseline, &percent.to_string(), style::TEXT);
            let bar_x = right - number_w - 12.0 - SLIDER_W;
            let bar = Rect::new(bar_x, top + (line / 2.0).floor() - 3.0, SLIDER_W, 6.0);
            style::bar(ui, bar, percent as f32 / 100.0, style::SWITCH_ON);
            if mouse_x >= bar.x - 8.0 && mouse_x <= bar.x + bar.w + 8.0 {
                let ratio = ((mouse_x - bar.x) / bar.w).clamp(0.0, 1.0);
                let stepped = (ratio * 100.0 / VOLUME_STEP as f32).round() as u32 * VOLUME_STEP;
                Tap::Volume(stepped)
            } else {
                Tap::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostalre_ui::context::UiContext;
    use nostalre_ui::state::StateCache;
    use nostalre_ui::test_support::test_frame;

    fn values() -> SettingsValues {
        SettingsValues {
            graphics: GraphicsSettings {
                ui_scale: 125.0,
                ..GraphicsSettings::default()
            },
            sound: SoundSettings {
                bgm_volume: 0.8,
                sfx_volume: 0.8,
                bgm_enabled: true,
                sfx_enabled: true,
                ..SoundSettings::default()
            },
            chat_lines: vec![String::new(); SLOT_COUNT],
            ..SettingsValues::default()
        }
    }

    fn loaded() -> Loaded {
        Loaded::new(values(), values())
    }

    /// Items waiting for its new key
    fn waiting() -> Settings {
        Settings {
            capturing: Some(HotkeyAction::ToggleInventory),
            ..Settings::default()
        }
    }

    /// One frame of the screen on `page`, `set` pressing what it presses
    fn frame_on(s: &mut Settings, keys: Keys, set: impl FnOnce(&mut UiContext)) -> Vec<GameEvent> {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(1280.0, 720.0);
        set(&mut ctx);
        let mut ui = test_frame(&mut ctx, &mut state);
        let mut events = Vec::new();
        s.build(
            &mut ui,
            Rect::new(64.0, 80.0, 760.0, 520.0),
            &keys,
            &mut events,
        );
        events
    }

    fn enter() -> Keys {
        Keys {
            enter: true,
            ..Keys::default()
        }
    }

    fn on_page(page: Page) -> Settings {
        let mut s = Settings {
            page,
            ..Settings::default()
        };
        s.set_values(values(), values());
        s
    }

    #[test]
    fn a_name_toggle_shows_on_while_its_hide_flag_is_off() {
        let mut v = values();
        assert!(matches!(
            Opt::NameNpc.shown_value(&mut v),
            Shown::Toggle(true)
        ));
        assert!(Opt::NameNpc.change(&mut v, Change::Prev));
        assert!(v.graphics.display.hide_name_npc);
        assert!(!Opt::NameNpc.change(&mut v, Change::Prev));
    }

    #[test]
    fn x_attacks_flips_and_sends_the_aim_event() {
        let mut v = values();
        assert!(matches!(Opt::AimAttack.shown_value(&mut v), Shown::Toggle(false)));
        assert!(Opt::AimAttack.change(&mut v, Change::Flip));
        assert!(v.aim_attack);
        assert_eq!(Opt::AimAttack.group(), Group::Aim);
    }

    #[test]
    fn scale_steps_stop_at_the_ends() {
        let mut v = values();
        v.graphics.ui_scale = 200.0;
        assert!(!Opt::Scale.change(&mut v, Change::Next));
        assert!(Opt::Scale.change(&mut v, Change::Prev));
        assert_eq!(v.graphics.ui_scale, 175.0);
        // a scale the steps do not have reads as the nearest
        v.graphics.ui_scale = 110.0;
        assert!(matches!(
            Opt::Scale.shown_value(&mut v),
            Shown::Choice("100%")
        ));
    }

    #[test]
    fn the_hud_choice_stops_at_its_ends() {
        let mut v = values();
        assert!(!Opt::Hud.change(&mut v, Change::Prev));
        assert!(Opt::Hud.change(&mut v, Change::Next));
        assert_eq!(v.hud, HudLayout::Classic);
        assert!(!Opt::Hud.change(&mut v, Change::Next));
    }

    #[test]
    fn a_volume_at_zero_is_off_and_back_up_is_on() {
        let mut v = values();
        v.sound.bgm_volume = 0.05;
        assert!(Opt::Music.change(&mut v, Change::Prev));
        assert!(!v.sound.bgm_enabled);
        assert!(Opt::Music.change(&mut v, Change::Next));
        assert!(v.sound.bgm_enabled);
        assert_eq!(percent(v.sound.bgm_volume, v.sound.bgm_enabled), 5);
        // a volume between the steps moves onto them
        v.sound.bgm_volume = 0.29;
        assert!(Opt::Music.change(&mut v, Change::Next));
        assert_eq!(percent(v.sound.bgm_volume, v.sound.bgm_enabled), 30);
        v.sound.bgm_volume = 0.29;
        assert!(Opt::Music.change(&mut v, Change::Prev));
        assert_eq!(percent(v.sound.bgm_volume, v.sound.bgm_enabled), 25);
    }

    #[test]
    fn a_volume_muted_elsewhere_comes_back_at_its_level() {
        let mut v = values();
        v.sound.bgm_enabled = false;
        assert!(matches!(Opt::Music.shown_value(&mut v), Shown::Slider(0)));
        assert!(Opt::Music.change(&mut v, Change::Next));
        assert_eq!(percent(v.sound.bgm_volume, v.sound.bgm_enabled), 80);
    }

    #[test]
    fn reset_puts_the_default_back() {
        let mut l = loaded();
        l.now.graphics.fog = !l.defaults.graphics.fog;
        assert!(Opt::Fog.reset(&mut l.now, &mut l.defaults));
        assert_eq!(l.now.graphics.fog, l.defaults.graphics.fog);
        assert!(!Opt::Fog.reset(&mut l.now, &mut l.defaults));
    }

    #[test]
    fn a_taken_key_swaps_and_a_reserved_one_is_refused() {
        let mut s = waiting();
        s.set_values(values(), values());
        // a bare letter belongs to chat
        assert!(
            s.capture(KeyChord::new("KeyI", false, false, false))
                .is_none()
        );
        assert!(s.is_capturing());
        let alt_q = KeyChord::new("KeyQ", true, false, false);
        let Some(GameEvent::KeyBindingsChanged(keys)) = s.capture(alt_q.clone()) else {
            panic!("no change");
        };
        assert_eq!(keys.get(HotkeyAction::ToggleInventory), Some(&alt_q));
        let alt_e = KeyChord::new("KeyE", true, false, false);
        assert_eq!(keys.get(HotkeyAction::ToggleEquipment), Some(&alt_e));
        assert!(!s.is_capturing());
    }

    #[test]
    fn a_key_back_to_its_default_swaps_too() {
        let mut l = loaded();
        let alt_e = KeyChord::new("KeyE", true, false, false);
        let alt_q = KeyChord::new("KeyQ", true, false, false);
        l.now.keys.set(HotkeyAction::ToggleInventory, alt_q.clone());
        l.now.keys.set(HotkeyAction::ToggleEquipment, alt_e.clone());
        assert!(reset_key(
            HotkeyAction::ToggleInventory,
            &mut l.now,
            &l.defaults
        ));
        assert_eq!(l.now.keys.get(HotkeyAction::ToggleInventory), Some(&alt_e));
        assert_eq!(l.now.keys.get(HotkeyAction::ToggleEquipment), Some(&alt_q));
    }

    #[test]
    fn escape_leaves_a_key_waiting_then_nothing() {
        let mut s = waiting();
        assert!(s.back());
        assert!(!s.back());
    }

    #[test]
    fn the_keep_tap_keeps_the_line_typed() {
        let mut s = on_page(Page::ChatLines);
        frame_on(&mut s, enter(), |_| {});
        assert!(s.is_editing());
        frame_on(&mut s, Keys::default(), |c| c.typed_chars = vec!['h', 'i']);
        let tap = Keys {
            enter: true,
            ..Keys::default()
        };
        let events = frame_on(&mut s, tap, |_| {});
        assert!(!s.is_editing());
        let [GameEvent::ShortcutBindingsChanged(lines)] = events.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(lines[0], "hi");
    }

    #[test]
    fn the_page_keys_do_not_drop_a_line_being_typed() {
        let mut s = on_page(Page::ChatLines);
        frame_on(&mut s, enter(), |_| {});
        frame_on(&mut s, Keys::default(), |c| c.typed_chars = vec!['a']);
        let tab = Keys {
            next_tab: true,
            ..Keys::default()
        };
        frame_on(&mut s, tab, |_| {});
        assert_eq!(s.page, Page::ChatLines);
        assert!(s.is_editing());
    }

    #[test]
    fn the_hud_row_sends_the_layout() {
        let mut s = on_page(Page::Display);
        s.picked = Page::Display.opts().position(|o| o == Opt::Hud).unwrap();
        let right = Keys {
            right: true,
            ..Keys::default()
        };
        let events = frame_on(&mut s, right, |_| {});
        assert!(matches!(
            events.as_slice(),
            [GameEvent::HudLayoutChanged {
                layout: HudLayout::Classic
            }]
        ));
    }

    #[test]
    fn reset_page_puts_every_option_back() {
        let mut s = on_page(Page::Sound);
        if let Some(l) = s.values.as_mut() {
            l.now.sound.stereo = !l.defaults.sound.stereo;
            l.now.sound.bgm_volume = 0.1;
        }
        // "Reset page" sits under the seven pages
        let y = 80.0 + PAGES.len() as f32 * ROW_H + 2.0 * SECTION_GAP + ROW_H / 2.0;
        let events = frame_on(&mut s, Keys::default(), |c| {
            (c.mouse_x, c.mouse_y, c.mouse_clicked) = (100.0, y, true)
        });
        let [GameEvent::SoundSettingsChanged { settings, .. }] = events.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(*settings, values().sound);
    }
}
