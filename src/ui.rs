use crate::engine::MeterLevel;
use crate::feedback;
use crate::listener_stats::{
    ConnectionProbeStatus, ConnectionTester, ListenerStats, MountStatus, StatsTarget,
};
use crate::output::OutputStatus;
use crate::settings::ThemePreference;
use crate::{Destination, Engine, Settings, Source, settings_load, settings_save};
use eframe::egui::{
    self, Align, Color32, ColorImage, ComboBox, Context, CornerRadius, FontData, FontDefinitions,
    FontFamily, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense, Shadow,
    Shape, Stroke, TextureHandle, TextureOptions, Ui as EguiUi, Vec2,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant};

const STRONG_FONT: &str = "strong";
const NARROW_WIDTH: f32 = 560.0;
const WIDE_WIDTH: f32 = 1100.0;
const MASTER_DETAIL_LEFT_WIDTH: f32 = 920.0;
const MASTER_DETAIL_RIGHT_WIDTH: f32 = 1160.0;
const FORM_MAX_WIDTH: f32 = 720.0;
const OVERVIEW_MAX_WIDTH: f32 = 820.0;

const FONT_CAPTION: f32 = 12.0;
const FONT_BODY: f32 = 13.5;
const FONT_TITLE_S: f32 = 15.0;
const FONT_TITLE_M: f32 = 16.5;
const FONT_TITLE_L: f32 = 18.0;

const SPACE_S: f32 = 4.0;
const SPACE_M: f32 = 8.0;
const SPACE_L: f32 = 12.0;

const SAVED_FEEDBACK_DURATION: Duration = Duration::from_secs(2);
const UNDO_TOAST_DURATION: Duration = Duration::from_secs(8);
const AUTOSAVE_DELAY: Duration = Duration::from_millis(500);
const MAX_ACTIVITY_EVENTS: usize = 30;
const SAVE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
const NEW_SOURCE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::N);

const FILENAME_FORMATS: &[(&str, &str)] = &[
    (
        "Datum Tid - Namn (standard)",
        crate::settings::DEFAULT_FILENAME_FORMAT,
    ),
    ("Namn – Datum – Tid", "{namn}-{datum}-{tid}"),
    ("Datum – Tid – Namn", "{datum}-{tid}-{namn}"),
    ("Datum – Namn", "{datum}-{namn}"),
    ("Namn – Datum", "{namn}-{datum}"),
    ("Namn – Unix-tid", "{namn}-{unix}"),
];

#[derive(Clone, Copy)]
struct Palette {
    canvas_bg: Color32,
    header_bg: Color32,
    card_bg: Color32,
    card_border: Color32,
    card_divider: Color32,
    pill_bg: Color32,
    pill_border: Color32,
    text_primary: Color32,
    text_secondary: Color32,
    text_disabled: Color32,
    btn_bg: Color32,
    btn_bg_hover: Color32,
    btn_bg_active: Color32,
    btn_border: Color32,
    btn_border_hover: Color32,
    btn_disabled_bg: Color32,
    btn_disabled_border: Color32,
    stream_accent: Color32,
    stream_active_bg: Color32,
    stream_active_hover: Color32,
    stream_active_border: Color32,
    stream_pill_bg: Color32,
    record_accent: Color32,
    record_active_bg: Color32,
    record_active_hover: Color32,
    record_active_border: Color32,
    record_pill_bg: Color32,
    warning: Color32,
    danger: Color32,
    warn_pill_bg: Color32,
    warn_pill_border: Color32,
    danger_pill_bg: Color32,
    danger_pill_border: Color32,
    segmented_bg: Color32,
    meter_track: Color32,
    meter_border: Color32,
    meter_ok: Color32,
    meter_warn: Color32,
    meter_peak: Color32,
    tab_active_bg: Color32,
    tab_active_fg: Color32,
    tab_active_border: Color32,
    card_shadow: Shadow,
}

impl Palette {
    fn for_theme(dark: bool) -> Self {
        if dark {
            Self {
                canvas_bg: Color32::from_rgb(15, 20, 28),
                header_bg: Color32::from_rgb(22, 29, 40),
                card_bg: Color32::from_rgb(27, 36, 50),
                card_border: Color32::from_rgb(47, 61, 82),
                card_divider: Color32::from_rgb(40, 52, 70),
                pill_bg: Color32::from_rgb(18, 24, 34),
                pill_border: Color32::from_rgb(44, 57, 77),
                text_primary: Color32::from_rgb(241, 245, 249),
                text_secondary: Color32::from_rgb(154, 169, 191),
                text_disabled: Color32::from_rgb(90, 105, 126),
                btn_bg: Color32::from_rgb(38, 50, 68),
                btn_bg_hover: Color32::from_rgb(49, 64, 87),
                btn_bg_active: Color32::from_rgb(60, 78, 105),
                btn_border: Color32::from_rgb(60, 77, 102),
                btn_border_hover: Color32::from_rgb(88, 110, 143),
                btn_disabled_bg: Color32::from_rgb(23, 31, 43),
                btn_disabled_border: Color32::from_rgb(38, 49, 66),
                stream_accent: Color32::from_rgb(45, 212, 191),
                stream_active_bg: Color32::from_rgb(15, 118, 110),
                stream_active_hover: Color32::from_rgb(13, 148, 136),
                stream_active_border: Color32::from_rgb(45, 212, 191),
                stream_pill_bg: Color32::from_rgb(17, 56, 56),
                record_accent: Color32::from_rgb(248, 113, 113),
                record_active_bg: Color32::from_rgb(185, 28, 28),
                record_active_hover: Color32::from_rgb(220, 38, 38),
                record_active_border: Color32::from_rgb(248, 113, 113),
                record_pill_bg: Color32::from_rgb(64, 24, 29),
                warning: Color32::from_rgb(251, 191, 36),
                danger: Color32::from_rgb(248, 113, 113),
                warn_pill_bg: Color32::from_rgb(45, 35, 18),
                warn_pill_border: Color32::from_rgb(180, 115, 22),
                danger_pill_bg: Color32::from_rgb(56, 22, 27),
                danger_pill_border: Color32::from_rgb(185, 45, 45),
                segmented_bg: Color32::from_rgb(16, 22, 31),
                meter_track: Color32::from_rgb(14, 19, 27),
                meter_border: Color32::from_rgb(42, 54, 72),
                meter_ok: Color32::from_rgb(45, 212, 191),
                meter_warn: Color32::from_rgb(251, 191, 36),
                meter_peak: Color32::from_rgb(239, 68, 68),
                tab_active_bg: Color32::from_rgb(20, 83, 84),
                tab_active_fg: Color32::from_rgb(240, 253, 250),
                tab_active_border: Color32::from_rgb(45, 212, 191),
                card_shadow: Shadow {
                    offset: [0, 2],
                    blur: 8,
                    spread: 0,
                    color: Color32::from_black_alpha(55),
                },
            }
        } else {
            Self {
                canvas_bg: Color32::from_rgb(238, 242, 247),
                header_bg: Color32::from_rgb(255, 255, 255),
                card_bg: Color32::from_rgb(255, 255, 255),
                card_border: Color32::from_rgb(203, 213, 225),
                card_divider: Color32::from_rgb(226, 232, 240),
                pill_bg: Color32::from_rgb(241, 245, 249),
                pill_border: Color32::from_rgb(203, 213, 225),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_secondary: Color32::from_rgb(71, 85, 105),
                text_disabled: Color32::from_rgb(148, 163, 184),
                btn_bg: Color32::from_rgb(248, 250, 252),
                btn_bg_hover: Color32::from_rgb(226, 232, 240),
                btn_bg_active: Color32::from_rgb(203, 213, 225),
                btn_border: Color32::from_rgb(203, 213, 225),
                btn_border_hover: Color32::from_rgb(148, 163, 184),
                btn_disabled_bg: Color32::from_rgb(241, 245, 249),
                btn_disabled_border: Color32::from_rgb(226, 232, 240),
                stream_accent: Color32::from_rgb(13, 148, 136),
                stream_active_bg: Color32::from_rgb(13, 148, 136),
                stream_active_hover: Color32::from_rgb(15, 118, 110),
                stream_active_border: Color32::from_rgb(15, 118, 110),
                stream_pill_bg: Color32::from_rgb(204, 251, 241),
                record_accent: Color32::from_rgb(220, 38, 38),
                record_active_bg: Color32::from_rgb(220, 38, 38),
                record_active_hover: Color32::from_rgb(185, 28, 28),
                record_active_border: Color32::from_rgb(185, 28, 28),
                record_pill_bg: Color32::from_rgb(254, 226, 226),
                warning: Color32::from_rgb(180, 83, 9),
                danger: Color32::from_rgb(220, 38, 38),
                warn_pill_bg: Color32::from_rgb(254, 243, 199),
                warn_pill_border: Color32::from_rgb(217, 119, 6),
                danger_pill_bg: Color32::from_rgb(254, 226, 226),
                danger_pill_border: Color32::from_rgb(220, 38, 38),
                segmented_bg: Color32::from_rgb(226, 232, 240),
                meter_track: Color32::from_rgb(226, 232, 240),
                meter_border: Color32::from_rgb(203, 213, 225),
                meter_ok: Color32::from_rgb(13, 148, 136),
                meter_warn: Color32::from_rgb(217, 119, 6),
                meter_peak: Color32::from_rgb(220, 38, 38),
                tab_active_bg: Color32::from_rgb(13, 148, 136),
                tab_active_fg: Color32::from_rgb(255, 255, 255),
                tab_active_border: Color32::from_rgb(15, 118, 110),
                card_shadow: Shadow {
                    offset: [0, 1],
                    blur: 4,
                    spread: 0,
                    color: Color32::from_black_alpha(18),
                },
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Icon {
    Help,
    ListenerStats,
    Theme,
    More,
    Broadcast,
    BroadcastStop,
    Record,
    RecordStop,
    Stop,
    Settings,
    Folder,
    Plus,
    Check,
    Back,
    Close,
    Lock,
    Refresh,
    ChevronRight,
    ChevronDown,
    Eye,
    EyeOff,
    Undo,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LayoutMode {
    Narrow,
    Standard,
    Wide,
}

impl LayoutMode {
    fn from_width(width: f32) -> Self {
        if width < NARROW_WIDTH {
            Self::Narrow
        } else if width >= WIDE_WIDTH {
            Self::Wide
        } else {
            Self::Standard
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EventSeverity {
    Stream,
    Record,
    Warning,
}

struct ActivityEvent {
    time: String,
    message: String,
    severity: EventSeverity,
}

enum UndoItem {
    Source {
        index: usize,
        source: Source,
    },
    Destination {
        index: usize,
        destination: Destination,
        linked_sources: Vec<(u64, usize)>,
    },
}

struct UndoToast {
    label: String,
    item: UndoItem,
    created_at: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Overview,
    SourceSettings,
    Destinations,
    ListenerStats,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StopConfirm {
    Streams,
    Recordings,
    Close { discard_changes: bool },
}

#[derive(Default)]
enum SaveState {
    #[default]
    Idle,
    Pending(Instant),
    Saved(Instant),
    Failed(String),
}

impl SaveState {
    fn changed(&mut self, now: Instant) {
        *self = Self::Pending(now + AUTOSAVE_DELAY);
    }

    fn dirty(&self) -> bool {
        matches!(self, Self::Pending(_) | Self::Failed(_))
    }

    fn due(&self, now: Instant) -> bool {
        matches!(self, Self::Pending(deadline) if now >= *deadline)
    }
}

pub struct Ui {
    saved_recordings: VecDeque<crate::recording_dialog::SavedRecording>,
    recording_errors: VecDeque<String>,
    engine: Engine,
    settings: Settings,
    selected_source: Option<u64>,
    view: View,
    listener_stats: ListenerStats,
    connection_tester: ConnectionTester,
    inline_edit_destination: Option<u64>,
    expanded_destinations: HashSet<u64>,
    undo_toast: Option<UndoToast>,
    activity_log: VecDeque<ActivityEvent>,
    last_logged_issues: HashMap<u64, Vec<String>>,
    save_state: SaveState,
    hidden_settings_error: Option<String>,
    new_destination_draft: Option<Destination>,
    confirm_stop: Option<StopConfirm>,
    app_icon: Option<TextureHandle>,
    close_blocked: bool,
    close_confirmed: bool,
    load_error: Option<String>,
}

impl Ui {
    pub fn new(ctx: &Context) -> Self {
        configure_fonts(ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        ctx.set_visuals_of(egui::Theme::Light, build_visuals(false));
        ctx.all_styles_mut(|style| {
            for font in style.text_styles.values_mut() {
                font.size += 1.5;
            }
            style.spacing.item_spacing = Vec2::new(10.0, 8.0);
            style.spacing.button_padding = Vec2::new(11.0, 6.0);
            style.spacing.interact_size.y = 28.0;
        });

        let app_icon = load_app_icon_texture(ctx);
        let ui = Self::from_settings(settings_load(), Engine::new(), app_icon);
        apply_theme_preference(ctx, ui.settings.theme);
        ui
    }

    fn from_settings(
        loaded: Result<Settings, String>,
        engine: Engine,
        app_icon: Option<TextureHandle>,
    ) -> Self {
        let (settings, load_error) = match loaded {
            Ok(settings) => (settings, None),
            Err(err) => (Settings::default(), Some(err)),
        };
        engine.update(&settings.sources);
        let selected_source = settings.sources.first().map(|source| source.id);
        let mut expanded_destinations = HashSet::new();
        if settings.destinations.len() == 1 {
            expanded_destinations.insert(settings.destinations[0].id);
        }
        Self {
            engine,
            settings,
            selected_source,
            view: View::Overview,
            listener_stats: ListenerStats::default(),
            connection_tester: ConnectionTester::default(),
            inline_edit_destination: None,
            expanded_destinations,
            undo_toast: None,
            activity_log: VecDeque::new(),
            last_logged_issues: HashMap::new(),
            save_state: SaveState::Idle,
            saved_recordings: Default::default(),
            recording_errors: VecDeque::new(),
            hidden_settings_error: None,
            new_destination_draft: None,
            confirm_stop: None,
            app_icon,
            close_blocked: false,
            close_confirmed: false,
            load_error,
        }
    }

    fn navigate(&mut self, view: View) {
        self.view = view;
        self.new_destination_draft = None;
        if view != View::SourceSettings {
            self.inline_edit_destination = None;
        }
    }

    fn open_source_settings(&mut self, id: u64) {
        self.selected_source = Some(id);
        self.inline_edit_destination = None;
        self.navigate(View::SourceSettings);
    }

    fn log_event(&mut self, severity: EventSeverity, message: impl Into<String>) {
        if self.activity_log.len() >= MAX_ACTIVITY_EVENTS {
            self.activity_log.pop_back();
        }
        self.activity_log.push_front(ActivityEvent {
            time: crate::output::local_hm(),
            message: message.into(),
            severity,
        });
    }

    fn sync_issue_events(&mut self) {
        let mut newly_reported = Vec::new();
        let active_ids: HashSet<u64> = self.settings.sources.iter().map(|s| s.id).collect();
        self.last_logged_issues
            .retain(|id, _| active_ids.contains(id));
        for source in &self.settings.sources {
            let live = self.engine.live_status(source.id);
            let issues: Vec<String> = live
                .targets
                .iter()
                .filter_map(|target| match &target.detail {
                    OutputStatus::Problem(reason) => Some(format!("{} · {reason}", target.name)),
                    OutputStatus::Progress(_) => None,
                })
                .collect();
            let prev = self.last_logged_issues.entry(source.id).or_default();
            for issue in &issues {
                if !prev.contains(issue) {
                    newly_reported.push(format!("{} · {issue}", source.display_name()));
                }
            }
            *prev = issues;
        }
        for message in newly_reported {
            self.log_event(EventSeverity::Warning, message);
        }
    }

    fn set_source_stream(&mut self, source_id: u64, start: bool) {
        let Some(source) = self.settings.sources.iter().find(|s| s.id == source_id) else {
            return;
        };
        let name = source.display_name().to_owned();
        self.engine
            .set_stream(source, &self.settings.destinations, start);
        self.log_event(
            EventSeverity::Stream,
            if start {
                format!("Sändning startad ({name})")
            } else {
                format!("Sändning stoppad ({name})")
            },
        );
    }

    fn set_source_record(&mut self, source_id: u64, start: bool) {
        let Some(source) = self.settings.sources.iter().find(|s| s.id == source_id) else {
            return;
        };
        let name = source.display_name().to_owned();
        let format = source.recording.format.clone();
        self.engine.set_record(source, start);
        self.log_event(
            EventSeverity::Record,
            if start {
                format!("Inspelning startad ({name} · {format})")
            } else {
                format!("Inspelning stoppad ({name})")
            },
        );
    }

    fn add_source(&mut self) {
        if self.load_error.is_some() {
            return;
        }
        let id = self.settings.next_source_id();
        self.settings.sources.push(Source {
            id,
            name: format!("Ljudkälla {id}"),
            destination_ids: Vec::new(),
            ..Source::default()
        });
        self.open_source_settings(id);
        self.save_state.changed(Instant::now());
        self.engine.update(&self.settings.sources);
    }

    fn add_destination(&mut self) {
        if self.load_error.is_some() {
            return;
        }
        let id = self.settings.next_destination_id();
        self.settings.destinations.push(Destination {
            id,
            name: format!("Destination {id}"),
            ..Destination::default()
        });
        self.expanded_destinations.insert(id);
        self.navigate(View::Destinations);
        self.save_state.changed(Instant::now());
    }

    fn delete_source(&mut self, index: usize) {
        let source = self.settings.sources.remove(index);
        let label = format!("Ljudkälla ”{}” borttagen", source.display_name());
        self.selected_source = self
            .settings
            .sources
            .get(index)
            .or_else(|| self.settings.sources.last())
            .map(|s| s.id);
        self.undo_toast = Some(UndoToast {
            label,
            item: UndoItem::Source { index, source },
            created_at: Instant::now(),
        });
        self.navigate(View::Overview);
        self.save_state.changed(Instant::now());
        self.engine.update(&self.settings.sources);
    }

    fn delete_destination(&mut self, dest_id: u64) {
        let Some(index) = self
            .settings
            .destinations
            .iter()
            .position(|dest| dest.id == dest_id)
        else {
            return;
        };
        let destination = self.settings.destinations.remove(index);
        let mut linked_sources = Vec::new();
        for source in &mut self.settings.sources {
            if let Some(pos) = source.destination_ids.iter().position(|id| *id == dest_id) {
                source.destination_ids.remove(pos);
                linked_sources.push((source.id, pos));
            }
        }
        self.expanded_destinations.remove(&dest_id);
        if self.inline_edit_destination == Some(dest_id) {
            self.inline_edit_destination = None;
        }
        self.connection_tester.clear(dest_id);
        let label = format!("Destination ”{}” borttagen", destination.display_name());
        self.undo_toast = Some(UndoToast {
            label,
            item: UndoItem::Destination {
                index,
                destination,
                linked_sources,
            },
            created_at: Instant::now(),
        });
        self.save_state.changed(Instant::now());
        self.engine.update(&self.settings.sources);
    }

    fn undo_delete(&mut self) {
        let Some(toast) = self.undo_toast.take() else {
            return;
        };
        match toast.item {
            UndoItem::Source { index, mut source } => {
                if self.settings.sources.iter().any(|s| s.id == source.id) {
                    source.id = self.settings.next_source_id();
                }
                let pos = index.min(self.settings.sources.len());
                let id = source.id;
                self.settings.sources.insert(pos, source);
                if self.view != View::SourceSettings {
                    self.selected_source = Some(id);
                }
            }
            UndoItem::Destination {
                index,
                mut destination,
                linked_sources,
            } => {
                if self
                    .settings
                    .destinations
                    .iter()
                    .any(|d| d.id == destination.id)
                {
                    destination.id = self.settings.next_destination_id();
                }
                let dest_id = destination.id;
                let pos = index.min(self.settings.destinations.len());
                self.settings.destinations.insert(pos, destination);
                for (source_id, slot_pos) in linked_sources {
                    if let Some(source) =
                        self.settings.sources.iter_mut().find(|s| s.id == source_id)
                        && !source.destination_ids.contains(&dest_id)
                    {
                        let insert_at = slot_pos.min(source.destination_ids.len());
                        source.destination_ids.insert(insert_at, dest_id);
                    }
                }
            }
        }
        self.save_state.changed(Instant::now());
        self.engine.update(&self.settings.sources);
    }

    fn destination_streaming(&self, destination_id: u64) -> bool {
        self.settings.sources.iter().any(|source| {
            source.destination_ids.contains(&destination_id) && self.engine.streaming(source.id)
        })
    }

    fn poll_listener_stats(
        &mut self,
        ctx: &Context,
        force_refresh: bool,
    ) -> Vec<Result<StatsTarget, &'static str>> {
        let targets: Vec<_> = self
            .settings
            .destinations
            .iter()
            .map(StatsTarget::parse)
            .collect();
        self.listener_stats.update(
            ctx,
            targets.iter().filter_map(|target| target.as_ref().ok()),
            force_refresh,
        );
        targets
    }

    fn wide_live_panel(&mut self, ctx: &Context, palette: Palette) {
        let mut force_refresh = false;
        let targets = self.poll_listener_stats(ctx, false);

        egui::SidePanel::right("wide_live_panel")
            .resizable(false)
            .exact_width(288.0)
            .frame(
                Frame::new()
                    .fill(palette.header_bg)
                    .stroke(Stroke::new(1.0, palette.card_divider))
                    .inner_margin(Margin::symmetric(14, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(strong_text(
                            "Live-panel",
                            FONT_TITLE_S,
                            palette.text_primary,
                        ));
                        ui.label(
                            RichText::new("Lyssnarstatistik & händelser")
                                .size(FONT_CAPTION)
                                .color(palette.text_secondary),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let refreshing = self.listener_stats.refreshing();
                        if icon_button(
                            ui,
                            Icon::Refresh,
                            !self.settings.destinations.is_empty() && !refreshing,
                            palette.text_secondary,
                            false,
                            palette,
                            "Uppdatera lyssnarstatistik",
                        )
                        .clicked()
                        {
                            force_refresh = true;
                        }
                        if refreshing {
                            ui.spinner();
                        }
                    });
                });

                ui.add_space(SPACE_M);

                // Listener Stats section
                card_frame(palette, palette.card_border)
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(strong_text("Lyssnare nu", FONT_BODY, palette.text_primary));
                        ui.add_space(SPACE_S);
                        if self.settings.destinations.is_empty() {
                            ui.label(
                                RichText::new("Lägg till en destination för att se lyssnare.")
                                    .size(FONT_CAPTION)
                                    .color(palette.text_secondary),
                            );
                        } else {
                            for (index, (destination, target)) in
                                self.settings.destinations.iter().zip(&targets).enumerate()
                            {
                                if index > 0 {
                                    ui.add_space(6.0);
                                    let (sep, _) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), 1.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        sep,
                                        CornerRadius::ZERO,
                                        palette.card_divider,
                                    );
                                    ui.add_space(6.0);
                                }
                                match target {
                                    Ok(target) => {
                                        let mount_label = format!(
                                            "{} ({})",
                                            destination.display_name(),
                                            target.mount
                                        );
                                        match self.listener_stats.status(target) {
                                            MountStatus::Listed(counts) => {
                                                let current = counts.current.map_or_else(
                                                    || "—".to_owned(),
                                                    |c| c.to_string(),
                                                );
                                                let peak = counts.peak.map_or_else(
                                                    || "—".to_owned(),
                                                    |p| p.to_string(),
                                                );
                                                ui.horizontal(|ui| {
                                                    ui.label(strong_text(
                                                        current,
                                                        22.0,
                                                        palette.text_primary,
                                                    ));
                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Center),
                                                        |ui| {
                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "Topp {peak}"
                                                                ))
                                                                .size(FONT_CAPTION)
                                                                .color(palette.stream_accent),
                                                            );
                                                        },
                                                    );
                                                });
                                                ui.label(
                                                    RichText::new(mount_label)
                                                        .size(FONT_CAPTION)
                                                        .color(palette.text_secondary),
                                                );
                                                if let Some(samples) =
                                                    self.listener_stats.history(target)
                                                {
                                                    ui.add_space(4.0);
                                                    paint_sparkline(ui, samples, palette);
                                                }
                                            }
                                            MountStatus::Loading => {
                                                ui.label(
                                                    RichText::new(destination.display_name())
                                                        .size(FONT_BODY)
                                                        .color(palette.text_primary),
                                                );
                                                ui.label(
                                                    RichText::new("Hämtar statistik…")
                                                        .size(FONT_CAPTION)
                                                        .color(palette.text_secondary),
                                                );
                                            }
                                            MountStatus::Absent => {
                                                ui.label(
                                                    RichText::new(mount_label)
                                                        .size(FONT_CAPTION)
                                                        .color(palette.text_secondary),
                                                );
                                                ui.label(
                                                    RichText::new("Ej aktiv på servern")
                                                        .size(FONT_CAPTION)
                                                        .color(palette.text_disabled),
                                                );
                                            }
                                            MountStatus::Unavailable(err) => {
                                                ui.label(
                                                    RichText::new(destination.display_name())
                                                        .size(FONT_BODY)
                                                        .color(palette.text_primary),
                                                );
                                                ui.label(
                                                    RichText::new(err)
                                                        .size(FONT_CAPTION)
                                                        .color(palette.warning),
                                                );
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        ui.label(
                                            RichText::new(destination.display_name())
                                                .size(FONT_BODY)
                                                .color(palette.text_primary),
                                        );
                                        ui.label(
                                            RichText::new(*err)
                                                .size(FONT_CAPTION)
                                                .color(palette.warning),
                                        );
                                    }
                                }
                            }
                        }
                    });

                ui.add_space(SPACE_M);

                // Activity & Status Log section
                card_frame(palette, palette.card_border)
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(strong_text(
                            "Status & händelser",
                            FONT_BODY,
                            palette.text_primary,
                        ));
                        ui.add_space(SPACE_S);
                        if self.activity_log.is_empty() {
                            ui.label(
                                RichText::new("Inga händelser ännu under den här sessionen.")
                                    .size(FONT_CAPTION)
                                    .color(palette.text_secondary),
                            );
                        } else {
                            ScrollArea::vertical()
                                .id_salt("wide_activity_log")
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 6.0;
                                    for event in &self.activity_log {
                                        let dot_color = match event.severity {
                                            EventSeverity::Stream => palette.stream_accent,
                                            EventSeverity::Record => palette.record_accent,
                                            EventSeverity::Warning => palette.warning,
                                        };
                                        ui.horizontal_top(|ui| {
                                            let (dot_rect, _) = ui.allocate_exact_size(
                                                Vec2::new(8.0, 15.0),
                                                Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                dot_rect.center(),
                                                3.0,
                                                dot_color,
                                            );
                                            ui.label(
                                                RichText::new(&event.time)
                                                    .monospace()
                                                    .size(FONT_CAPTION)
                                                    .color(palette.text_secondary),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&event.message)
                                                        .size(FONT_CAPTION)
                                                        .color(palette.text_primary),
                                                )
                                                .wrap(),
                                            );
                                        });
                                    }
                                });
                        }
                    });
            });

        if force_refresh {
            let _ = self.poll_listener_stats(ctx, true);
        }
    }

    fn source_master_rail(&mut self, ctx: &Context, palette: Palette) {
        let mut open_settings_id = None;
        let mut add_new_source = false;

        egui::SidePanel::left("source_master_rail")
            .resizable(false)
            .exact_width(214.0)
            .frame(
                Frame::new()
                    .fill(palette.header_bg)
                    .stroke(Stroke::new(1.0, palette.card_divider))
                    .inner_margin(Margin::symmetric(12, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(strong_text("Ljudkällor", FONT_TITLE_S, palette.text_primary));
                    crate::guide::tip(
                        ui,
                        "Välj en ljudkälla för att öppna dess inställningar. Mätaren visar signalen i realtid.",
                    );
                });
                ui.add_space(SPACE_M);

                let list_height = (ui.available_height() - 42.0).max(80.0);
                ScrollArea::vertical()
                    .id_salt("master_source_list")
                    .max_height(list_height)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        for source in &self.settings.sources {
                            let selected = self.selected_source == Some(source.id);
                            let streaming = self.engine.streaming(source.id);
                            let recording = self.engine.recording(source.id);
                            let present = self.engine.present(source.id);
                            let live = self.engine.live_status(source.id);
                            let mut levels = self.engine.levels(source.id);
                            if !present {
                                for level in &mut levels {
                                    level.signal = 0.0;
                                }
                            }
                            let stereo = source.right.is_some();

                            let bg = if selected {
                                palette.stream_pill_bg
                            } else {
                                palette.card_bg
                            };
                            let border = if selected {
                                palette.stream_active_border
                            } else {
                                palette.card_border
                            };

                            let frame_resp = Frame::new()
                                .fill(bg)
                                .stroke(Stroke::new(1.0, border))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    ui.horizontal(|ui| {
                                        let dot_color = if recording {
                                            palette.record_accent
                                        } else if streaming {
                                            palette.stream_accent
                                        } else if present {
                                            palette.text_secondary
                                        } else {
                                            palette.text_disabled
                                        };
                                        let (dot_rect, _) = ui.allocate_exact_size(
                                            Vec2::new(8.0, 14.0),
                                            Sense::hover(),
                                        );
                                        ui.painter().circle_filled(
                                            dot_rect.center(),
                                            3.5,
                                            dot_color,
                                        );
                                        ui.add(
                                            egui::Label::new(strong_text(
                                                source.display_name(),
                                                FONT_BODY,
                                                palette.text_primary,
                                            ))
                                            .truncate(),
                                        );
                                    });

                                    let subtitle = if streaming && live.connected > 0 {
                                        format!("Sänder {}", format_hms(live.stream_elapsed))
                                    } else if recording {
                                        format!("Spelar in {}", format_hms(live.elapsed))
                                    } else if streaming {
                                        "Ansluter…".to_owned()
                                    } else if present {
                                        "Redo".to_owned()
                                    } else if source.device.is_empty() {
                                        "Ingen enhet".to_owned()
                                    } else {
                                        "Väntar på ljud".to_owned()
                                    };
                                    ui.label(
                                        RichText::new(subtitle)
                                            .size(11.5)
                                            .color(if streaming {
                                                palette.stream_accent
                                            } else if recording {
                                                palette.record_accent
                                            } else {
                                                palette.text_secondary
                                            }),
                                    );
                                    compact_meter(
                                        ui,
                                        &levels[..if stereo { 2 } else { 1 }],
                                        palette,
                                    );
                                });
                            let hit = ui.interact(
                                frame_resp.response.rect,
                                ui.id().with(("master-source", source.id)),
                                Sense::click(),
                            );
                            if hit.clicked() && !selected {
                                open_settings_id = Some(source.id);
                            }
                        }
                    });

                ui.add_space(SPACE_M);
                if action_button(
                    ui,
                    Icon::Plus,
                    "Lägg till ljudkälla",
                    true,
                    palette.text_primary,
                    ButtonVariant::Neutral,
                    palette,
                )
                .clicked()
                {
                    add_new_source = true;
                }
            });

        if let Some(id) = open_settings_id {
            self.open_source_settings(id);
        }
        if add_new_source {
            self.add_source();
        }
    }

    fn source_live_control_panel(&mut self, ctx: &Context, palette: Palette) {
        let Some(source) = self
            .settings
            .sources
            .iter()
            .find(|s| Some(s.id) == self.selected_source)
            .cloned()
        else {
            return;
        };
        let streaming = self.engine.streaming(source.id);
        let recording = self.engine.recording(source.id);
        let present = self.engine.present(source.id);
        let live = self.engine.live_status(source.id);
        let mut levels = self.engine.levels(source.id);
        if !present {
            for level in &mut levels {
                level.signal = 0.0;
            }
        }
        let stereo = source.right.is_some();
        let dest_count = source
            .resolve_destinations(&self.settings.destinations)
            .len();
        let mut toggle_stream = false;
        let mut toggle_record = false;
        let mut reset_clip_channel: Option<usize> = None;

        let targets = self.poll_listener_stats(ctx, false);

        egui::SidePanel::right("source_live_control")
            .resizable(false)
            .exact_width(236.0)
            .frame(
                Frame::new()
                    .fill(palette.header_bg)
                    .stroke(Stroke::new(1.0, palette.card_divider))
                    .inner_margin(Margin::symmetric(14, 12)),
            )
            .show(ctx, |ui| {
                ui.label(strong_text(
                    "Live-kontroll",
                    FONT_TITLE_S,
                    palette.text_primary,
                ));
                ui.label(
                    RichText::new(source.display_name())
                        .size(FONT_CAPTION)
                        .color(palette.text_secondary),
                );
                ui.add_space(SPACE_M);

                // Vertical stereo / mono broadcast meter
                card_frame(palette, palette.card_border)
                    .inner_margin(Margin::symmetric(10, 12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        vertical_stereo_meter(ui, &levels[..if stereo { 2 } else { 1 }], palette);
                        ui.add_space(SPACE_S);
                        let peak_signal = if stereo {
                            levels[0].signal.max(levels[1].signal)
                        } else {
                            levels[0].signal
                        };
                        let db_text = format_dbfs(peak_signal);
                        ui.vertical_centered(|ui| {
                            ui.label(strong_text(db_text, 18.0, palette.text_primary));
                        });
                        for (channel, level) in
                            levels.iter().take(if stereo { 2 } else { 1 }).enumerate()
                        {
                            if level.clipped {
                                let label = if stereo && channel == 0 {
                                    "Klippning V · Återställ"
                                } else if stereo {
                                    "Klippning H · Återställ"
                                } else {
                                    "Klippning · Återställ"
                                };
                                ui.add_space(4.0);
                                if compact_button(ui, label, true, Some(palette.danger), palette)
                                    .clicked()
                                {
                                    reset_clip_channel = Some(channel);
                                }
                            }
                        }
                    });

                ui.add_space(SPACE_M);

                // Stacked live action buttons
                let stream_enabled = streaming || (!source.device.is_empty() && dest_count > 0);
                let stream_reason = source_stream_disabled_reason(&source, dest_count, streaming);
                let (stream_icon, stream_label, stream_variant) = if streaming {
                    (Icon::Stop, "Stoppa sändning", ButtonVariant::StreamActive)
                } else {
                    (Icon::Broadcast, "Starta sändning", ButtonVariant::Neutral)
                };
                if with_disabled_reason(
                    full_width_action_button(
                        ui,
                        stream_icon,
                        stream_label,
                        stream_enabled,
                        palette.stream_accent,
                        stream_variant,
                        palette,
                    ),
                    stream_enabled,
                    stream_reason,
                )
                .clicked()
                {
                    toggle_stream = true;
                }

                ui.add_space(6.0);

                let record_enabled = recording
                    || (!source.device.is_empty() && !source.recording.folder.trim().is_empty());
                let record_reason = source_record_disabled_reason(&source, recording);
                let (record_icon, record_label, record_variant) = if recording {
                    (Icon::Stop, "Stoppa inspelning", ButtonVariant::RecordActive)
                } else {
                    (Icon::Record, "Starta inspelning", ButtonVariant::Neutral)
                };
                if with_disabled_reason(
                    full_width_action_button(
                        ui,
                        record_icon,
                        record_label,
                        record_enabled,
                        palette.record_accent,
                        record_variant,
                        palette,
                    ),
                    record_enabled,
                    record_reason,
                )
                .clicked()
                {
                    toggle_record = true;
                }

                // Active destination & listener summary
                if !source.destination_ids.is_empty() {
                    ui.add_space(SPACE_L);
                    ui.label(
                        RichText::new("Destinationer")
                            .size(FONT_CAPTION)
                            .color(palette.text_secondary),
                    );
                    ui.add_space(4.0);
                    for dest_id in &source.destination_ids {
                        if let Some(pos) = self
                            .settings
                            .destinations
                            .iter()
                            .position(|d| d.id == *dest_id)
                        {
                            let dest = &self.settings.destinations[pos];
                            let target_status = live
                                .targets
                                .iter()
                                .find(|ts| ts.name == dest.display_name());
                            let listener_text = targets
                                .get(pos)
                                .and_then(|t| t.as_ref().ok())
                                .and_then(|t| match self.listener_stats.status(t) {
                                    MountStatus::Listed(c) => c.current,
                                    _ => None,
                                })
                                .map(|n| format!(" · {n} lyssnare"))
                                .unwrap_or_default();
                            let (dot, bg, border, fg) = match target_status {
                                Some(ts)
                                    if ts.sending
                                        && matches!(ts.detail, OutputStatus::Progress(_)) =>
                                {
                                    (
                                        palette.stream_accent,
                                        palette.stream_pill_bg,
                                        palette.stream_active_border,
                                        palette.stream_accent,
                                    )
                                }
                                Some(_) => (
                                    palette.warning,
                                    palette.warn_pill_bg,
                                    palette.warn_pill_border,
                                    palette.warning,
                                ),
                                None => (
                                    palette.text_disabled,
                                    palette.pill_bg,
                                    palette.pill_border,
                                    palette.text_secondary,
                                ),
                            };
                            dot_pill(
                                ui,
                                &format!("{}{}", dest.display_name(), listener_text),
                                dot,
                                bg,
                                border,
                                fg,
                            );
                            ui.add_space(4.0);
                        }
                    }
                }
            });

        if let Some(channel) = reset_clip_channel {
            self.engine.reset_clip(source.id, channel);
        }
        if toggle_stream {
            self.set_source_stream(source.id, !streaming);
        }
        if toggle_record {
            self.set_source_record(source.id, !recording);
        }
    }

    fn undo_toast_overlay(&mut self, ctx: &Context, palette: Palette) {
        let Some(toast) = &self.undo_toast else {
            return;
        };
        if toast.created_at.elapsed() >= UNDO_TOAST_DURATION {
            self.undo_toast = None;
            return;
        }
        let label = toast.label.clone();
        let mut trigger_undo = false;
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("undo-toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0, palette.stream_active_border))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 8))
                    .shadow(palette.card_shadow)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.label(
                                RichText::new(&label)
                                    .size(FONT_BODY)
                                    .color(palette.text_primary),
                            );
                            if action_button(
                                ui,
                                Icon::Undo,
                                "Ångra",
                                true,
                                palette.tab_active_fg,
                                ButtonVariant::Primary,
                                palette,
                            )
                            .clicked()
                            {
                                trigger_undo = true;
                            }
                            if icon_button(
                                ui,
                                Icon::Close,
                                true,
                                palette.text_secondary,
                                false,
                                palette,
                                "Stäng",
                            )
                            .clicked()
                            {
                                dismiss = true;
                            }
                        });
                    });
            });

        if trigger_undo {
            self.undo_delete();
        } else if dismiss {
            self.undo_toast = None;
        }
    }

    fn save(&mut self) {
        match settings_save(&self.settings) {
            Ok(()) => {
                self.close_blocked = false;
                self.hidden_settings_error = None;
                self.save_state = SaveState::Saved(Instant::now());
            }
            Err(err) => self.save_state = SaveState::Failed(err),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if self.load_error.is_some()
            || self.confirm_stop.is_some()
            || !self.saved_recordings.is_empty()
        {
            return;
        }
        if ctx.input_mut(|input| input.consume_shortcut(&SAVE_SHORTCUT)) && self.save_state.dirty()
        {
            self.hidden_settings_error = None;
            self.save();
        }
        if ctx.input_mut(|input| input.consume_shortcut(&NEW_SOURCE_SHORTCUT)) {
            self.add_source();
        }
    }

    fn repaint_interval(&self) -> Duration {
        let active = self.settings.sources.iter().any(|source| {
            self.engine.present(source.id)
                || self.engine.streaming(source.id)
                || self.engine.recording(source.id)
        });
        if active {
            Duration::from_millis(65)
        } else {
            Duration::from_millis(500)
        }
    }

    fn header_stream_disabled_reason(&self, can_start_streams: bool) -> Option<&'static str> {
        if can_start_streams {
            return None;
        }
        if self.settings.sources.is_empty() {
            return Some("Lägg till minst en ljudkälla");
        }
        let idle_sources: Vec<&Source> = self
            .settings
            .sources
            .iter()
            .filter(|source| !self.engine.streaming(source.id))
            .collect();
        if idle_sources.is_empty() {
            return Some("Alla ljudkällor sänder redan");
        }
        if idle_sources.iter().all(|source| source.device.is_empty()) {
            Some("Välj en ljudenhet i inställningarna")
        } else {
            Some("Lägg till minst en destination")
        }
    }

    fn header_record_disabled_reason(&self, can_start_recordings: bool) -> Option<&'static str> {
        if can_start_recordings {
            return None;
        }
        if self.settings.sources.is_empty() {
            return Some("Lägg till minst en ljudkälla");
        }
        let idle_sources: Vec<&Source> = self
            .settings
            .sources
            .iter()
            .filter(|source| !self.engine.recording(source.id))
            .collect();
        if idle_sources.is_empty() {
            return Some("Alla ljudkällor spelar redan in");
        }
        if idle_sources.iter().all(|source| source.device.is_empty()) {
            Some("Välj en ljudenhet i inställningarna")
        } else {
            Some("Välj en inspelningsmapp")
        }
    }

    fn stop_confirm_modal(&mut self, ctx: &Context, palette: Palette) {
        let Some(kind) = self.confirm_stop else {
            return;
        };
        let sources = &self.settings.sources;
        let streams = sources
            .iter()
            .filter(|s| self.engine.streaming(s.id))
            .count();
        let recordings = sources
            .iter()
            .filter(|s| self.engine.recording(s.id))
            .count();

        let (title, body) = match kind {
            StopConfirm::Streams if streams > 0 => (
                "Stoppa alla sändningar?",
                if streams == 1 {
                    "1 pågående sändning kommer att stoppas.".to_owned()
                } else {
                    format!("{streams} pågående sändningar kommer att stoppas.")
                },
            ),
            StopConfirm::Recordings if recordings > 0 => (
                "Stoppa alla inspelningar?",
                if recordings == 1 {
                    "1 pågående inspelning kommer att stoppas.".to_owned()
                } else {
                    format!("{recordings} pågående inspelningar kommer att stoppas.")
                },
            ),
            StopConfirm::Close { .. } => (
                "Stäng Mötessändaren?",
                match (streams > 0, recordings > 0) {
                    (true, true) => "Pågående sändningar och inspelningar stoppas.",
                    (true, false) => "Pågående sändningar stoppas.",
                    (false, true) => "Pågående inspelningar stoppas.",
                    (false, false) => "Sändningar och inspelningar har avslutats.",
                }
                .to_owned(),
            ),
            _ => {
                self.confirm_stop = None;
                return;
            }
        };

        let mut confirmed = false;
        let mut cancelled = false;
        let closing = matches!(kind, StopConfirm::Close { .. });
        let modal = egui::Modal::new(egui::Id::new(("confirm-stop-modal", closing)))
            .frame(
                Frame::new()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0, palette.card_border))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(18, 16))
                    .shadow(palette.card_shadow),
            )
            .show(ctx, |ui| {
                ui.set_width((ctx.screen_rect().width() - 60.0).clamp(1.0, 360.0));
                ui.label(strong_text(title, 16.0, palette.text_primary));
                ui.add_space(6.0);
                ui.label(RichText::new(&body).color(palette.text_secondary));
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if action_button(
                        ui,
                        Icon::Stop,
                        if closing { "Stäng" } else { "Stoppa" },
                        true,
                        Color32::WHITE,
                        ButtonVariant::Danger,
                        palette,
                    )
                    .clicked()
                    {
                        confirmed = true;
                    }
                    if text_button(ui, "Avbryt", true, ButtonVariant::Neutral, palette).clicked() {
                        cancelled = true;
                    }
                });
            });

        if confirmed {
            match kind {
                StopConfirm::Close { discard_changes } => {
                    if discard_changes {
                        self.save_state = SaveState::Idle;
                    }
                    self.close_confirmed = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                StopConfirm::Streams => {
                    let ids: Vec<u64> = self
                        .settings
                        .sources
                        .iter()
                        .filter(|s| self.engine.streaming(s.id))
                        .map(|s| s.id)
                        .collect();
                    for id in ids {
                        self.set_source_stream(id, false);
                    }
                }
                StopConfirm::Recordings => {
                    let ids: Vec<u64> = self
                        .settings
                        .sources
                        .iter()
                        .filter(|s| self.engine.recording(s.id))
                        .map(|s| s.id)
                        .collect();
                    for id in ids {
                        self.set_source_record(id, false);
                    }
                }
            }
            self.confirm_stop = None;
        } else if cancelled || modal.should_close() {
            self.confirm_stop = None;
        }
    }

    fn theme_menu_contents(&mut self, ui: &mut EguiUi, palette: Palette) {
        ui.label(strong_text("Utseende", FONT_BODY, palette.text_primary));
        ui.add_space(4.0);
        for (pref, label) in [
            (ThemePreference::System, "Följ systemet"),
            (ThemePreference::Light, "Ljust"),
            (ThemePreference::Dark, "Mörkt"),
        ] {
            let selected = self.settings.theme == pref;
            if ui.selectable_label(selected, label).clicked() {
                if !selected {
                    self.settings.theme = pref;
                    apply_theme_preference(ui.ctx(), pref);
                    self.save_state.changed(Instant::now());
                }
                ui.close_menu();
            }
        }
    }

    fn header(&mut self, ui: &mut EguiUi, mode: LayoutMode, palette: Palette) {
        let total_sources = self.settings.sources.len();
        let streaming_count = self
            .settings
            .sources
            .iter()
            .filter(|source| self.engine.streaming(source.id))
            .count();
        let recording_count = self
            .settings
            .sources
            .iter()
            .filter(|source| self.engine.recording(source.id))
            .count();
        let can_start_streams = self.settings.sources.iter().any(|source| {
            !source.device.is_empty()
                && !source
                    .resolve_destinations(&self.settings.destinations)
                    .is_empty()
                && !self.engine.streaming(source.id)
        });
        let can_stop_streams = streaming_count > 0;
        let can_start_recordings = self.settings.sources.iter().any(|source| {
            !source.device.is_empty()
                && !source.recording.folder.trim().is_empty()
                && !self.engine.recording(source.id)
        });
        let can_stop_recordings = recording_count > 0;
        let stream_disabled_reason = self.header_stream_disabled_reason(can_start_streams);
        let record_disabled_reason = self.header_record_disabled_reason(can_start_recordings);
        let narrow = mode == LayoutMode::Narrow;

        let mut start_all_streams = false;
        let mut start_all_recordings = false;

        ui.horizontal(|ui| {
            ui.set_min_height(36.0);
            ui.spacing_mut().item_spacing.x = if narrow { 6.0 } else { 10.0 };
            if let Some(icon) = &self.app_icon {
                ui.add(
                    egui::Image::new((icon.id(), Vec2::splat(26.0)))
                        .corner_radius(CornerRadius::same(6)),
                );
            }
            if !narrow {
                ui.label(strong_text(
                    "Mötessändare",
                    FONT_TITLE_L,
                    palette.text_primary,
                ));
                let help = crate::guide::enabled(ui.ctx());
                if icon_button(
                    ui,
                    Icon::Help,
                    true,
                    palette.text_secondary,
                    help,
                    palette,
                    if help {
                        "Stäng hjälptipsen"
                    } else {
                        "Visa hjälptips vid knappar och fält"
                    },
                )
                .clicked()
                {
                    crate::guide::set_enabled(ui.ctx(), !help);
                }
                let showing_stats = self.view == View::ListenerStats;
                if icon_button(
                    ui,
                    Icon::ListenerStats,
                    self.load_error.is_none(),
                    palette.text_secondary,
                    showing_stats,
                    palette,
                    if showing_stats {
                        "Stäng lyssnarstatistiken"
                    } else {
                        "Visa lyssnarstatistik"
                    },
                )
                .clicked()
                {
                    self.navigate(if showing_stats {
                        View::Overview
                    } else {
                        View::ListenerStats
                    });
                }

                // Theme selector popover
                let theme_popup_id = ui.make_persistent_id("theme-selector-popup");
                let theme_open = ui.memory(|m| m.is_popup_open(theme_popup_id));
                let theme_btn = icon_button(
                    ui,
                    Icon::Theme,
                    true,
                    palette.text_secondary,
                    theme_open,
                    palette,
                    "Välj utseende (System / Ljust / Mörkt)",
                );
                if theme_btn.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(theme_popup_id));
                }
                egui::popup_below_widget(
                    ui,
                    theme_popup_id,
                    &theme_btn,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(148.0);
                        self.theme_menu_contents(ui, palette);
                    },
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.set_height(36.0);
                ui.spacing_mut().item_spacing.x = 6.0;

                // Overflow menu on narrow docked windows
                if narrow {
                    let overflow_id = ui.make_persistent_id("narrow-overflow-popup");
                    let overflow_open = ui.memory(|m| m.is_popup_open(overflow_id));
                    let more_btn = icon_button(
                        ui,
                        Icon::More,
                        true,
                        palette.text_secondary,
                        overflow_open,
                        palette,
                        "Fler alternativ",
                    );
                    if more_btn.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(overflow_id));
                    }
                    egui::popup_below_widget(
                        ui,
                        overflow_id,
                        &more_btn,
                        egui::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            ui.set_min_width(176.0);
                            let help = crate::guide::enabled(ui.ctx());
                            if ui
                                .selectable_label(
                                    help,
                                    if help {
                                        "Dölj hjälptips"
                                    } else {
                                        "Visa hjälptips"
                                    },
                                )
                                .clicked()
                            {
                                crate::guide::set_enabled(ui.ctx(), !help);
                                ui.close_menu();
                            }
                            let showing_stats = self.view == View::ListenerStats;
                            if ui
                                .add_enabled(
                                    self.load_error.is_none(),
                                    egui::SelectableLabel::new(showing_stats, "Lyssnarstatistik"),
                                )
                                .clicked()
                            {
                                self.navigate(if showing_stats {
                                    View::Overview
                                } else {
                                    View::ListenerStats
                                });
                                ui.close_menu();
                            }
                            ui.separator();
                            self.theme_menu_contents(ui, palette);
                        },
                    );
                }

                let show_group_names = !narrow && ui.available_width() >= 550.0;
                // Recording bulk controls pill + live count (far right).
                header_count(
                    ui,
                    recording_count,
                    total_sources,
                    palette.record_accent,
                    palette,
                );
                let (record_start, record_stop) = header_pill(ui, "record-pill", palette, |ui| {
                    let start = with_disabled_reason(
                        icon_button(
                            ui,
                            Icon::Record,
                            can_start_recordings,
                            palette.record_accent,
                            false,
                            palette,
                            "Starta alla inspelningar",
                        ),
                        can_start_recordings,
                        record_disabled_reason,
                    );
                    let stop = icon_button(
                        ui,
                        Icon::RecordStop,
                        can_stop_recordings,
                        palette.record_accent,
                        false,
                        palette,
                        "Stoppa alla inspelningar",
                    );
                    (start, stop)
                });
                if show_group_names {
                    ui.small("Inspelning");
                }
                if record_start.clicked() {
                    start_all_recordings = true;
                }
                if record_stop.clicked() {
                    self.confirm_stop = Some(StopConfirm::Recordings);
                }

                ui.add_space(if narrow { 2.0 } else { 6.0 });

                // Streaming bulk controls pill + live count (left of recording group).
                header_count(
                    ui,
                    streaming_count,
                    total_sources,
                    palette.stream_accent,
                    palette,
                );
                let (stream_start, stream_stop) = header_pill(ui, "stream-pill", palette, |ui| {
                    let start = with_disabled_reason(
                        icon_button(
                            ui,
                            Icon::Broadcast,
                            can_start_streams,
                            palette.stream_accent,
                            false,
                            palette,
                            "Starta alla sändningar",
                        ),
                        can_start_streams,
                        stream_disabled_reason,
                    );
                    let stop = icon_button(
                        ui,
                        Icon::BroadcastStop,
                        can_stop_streams,
                        palette.stream_accent,
                        false,
                        palette,
                        "Stoppa alla sändningar",
                    );
                    (start, stop)
                });
                if show_group_names {
                    ui.small("Sändning");
                }
                if stream_start.clicked() {
                    start_all_streams = true;
                }
                if !narrow && ui.available_width() >= 100.0 {
                    ui.label(
                        RichText::new("Alla ljudkällor")
                            .size(FONT_CAPTION)
                            .color(palette.text_secondary),
                    );
                }
                if stream_stop.clicked() {
                    self.confirm_stop = Some(StopConfirm::Streams);
                }
            });
        });

        if start_all_streams {
            let ids: Vec<u64> = self
                .settings
                .sources
                .iter()
                .filter(|source| {
                    !source.device.is_empty()
                        && !source
                            .resolve_destinations(&self.settings.destinations)
                            .is_empty()
                        && !self.engine.streaming(source.id)
                })
                .map(|s| s.id)
                .collect();
            for id in ids {
                self.set_source_stream(id, true);
            }
        }
        if start_all_recordings {
            let ids: Vec<u64> = self
                .settings
                .sources
                .iter()
                .filter(|source| {
                    !source.device.is_empty()
                        && !source.recording.folder.trim().is_empty()
                        && !self.engine.recording(source.id)
                })
                .map(|s| s.id)
                .collect();
            for id in ids {
                self.set_source_record(id, true);
            }
        }

        if !self.recording_errors.is_empty() || matches!(self.save_state, SaveState::Failed(_)) {
            ui.add_space(SPACE_M);
            ScrollArea::vertical()
                .id_salt("app-errors")
                .max_height(ui.ctx().screen_rect().height() * 0.30)
                .show(ui, |ui| self.app_errors(ui, palette));
        }
    }

    fn app_errors(&mut self, ui: &mut EguiUi, palette: Palette) {
        if let SaveState::Failed(error) = &self.save_state {
            ui.strong("Inställningarna kunde inte sparas");
            settings_error_detail(ui, error, &mut self.hidden_settings_error);
            let mut retry = false;
            let mut close_without_save = false;
            ui.horizontal_wrapped(|ui| {
                if compact_button(ui, "Försök igen", true, None, palette).clicked() {
                    retry = true;
                }
                if self.close_blocked
                    && compact_button(ui, "Stäng utan att spara", true, None, palette).clicked()
                {
                    close_without_save = true;
                }
            });
            if retry {
                self.hidden_settings_error = None;
                self.save();
            } else if close_without_save {
                if self.has_active_outputs() {
                    self.confirm_stop = Some(StopConfirm::Close {
                        discard_changes: true,
                    });
                } else {
                    self.save_state = SaveState::Idle;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if let Some(error) = self.recording_errors.front() {
            let count = self.recording_errors.len();
            if count > 1 {
                ui.strong(format!("Inspelningsfel (1 av {count})"));
            }
            if feedback::message(ui, error) {
                self.recording_errors.pop_front();
            }
        }
    }

    fn load_error_page(&mut self, ui: &mut EguiUi) {
        ui.heading("Inställningarna kunde inte läsas in");
        ui.label("När inställningarna har lästs in kan du använda dina ljudkällor igen.");
        if let Some(error) = &self.load_error {
            settings_error_detail(ui, error, &mut self.hidden_settings_error);
        }
        if ui.button("Läs in igen").clicked() {
            self.hidden_settings_error = None;
            match settings_load() {
                Ok(settings) => {
                    apply_theme_preference(ui.ctx(), settings.theme);
                    self.selected_source = settings.sources.first().map(|source| source.id);
                    self.settings = settings;
                    self.engine.update(&self.settings.sources);
                    self.load_error = None;
                }
                Err(error) => {
                    self.load_error = Some(error);
                }
            }
        }
    }

    fn collect_recordings(&mut self) {
        for (source, result) in self.engine.completed_recordings() {
            match result {
                Ok(path) => {
                    self.log_event(
                        EventSeverity::Record,
                        format!("Inspelning sparad ({source})"),
                    );
                    self.saved_recordings
                        .push_back(crate::recording_dialog::SavedRecording::new(source, path));
                }
                Err(error) => {
                    let msg = format!("Inspelningen för {source} misslyckades. {error}");
                    self.log_event(EventSeverity::Warning, msg.clone());
                    self.recording_errors.push_back(msg);
                }
            }
        }
    }

    fn has_active_outputs(&self) -> bool {
        self.settings
            .sources
            .iter()
            .any(|s| self.engine.streaming(s.id) || self.engine.recording(s.id))
    }
}

impl eframe::App for Ui {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let errors = self.engine.shutdown();
        if !errors.is_empty() {
            rfd::MessageDialog::new()
                .set_title("Fel när inspelningen skulle sparas")
                .set_description(errors.join("\n\n"))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let palette = Palette::for_theme(ctx.style().visuals.dark_mode);
        let screen_width = ctx.screen_rect().width();
        let mode = LayoutMode::from_width(screen_width);

        self.connection_tester.poll();
        self.sync_issue_events();

        let closing = ctx.input(|input| input.viewport().close_requested());
        if !closing {
            self.collect_recordings();
            self.handle_shortcuts(ctx);
        }

        egui::TopBottomPanel::top("header")
            .frame(
                Frame::new()
                    .fill(palette.header_bg)
                    .inner_margin(Margin::symmetric(
                        if mode == LayoutMode::Narrow { 10 } else { 16 },
                        10,
                    ))
                    .stroke(Stroke::new(1.0, palette.card_border)),
            )
            .show(ctx, |ui| {
                self.header(ui, mode, palette);
            });

        let show_wide_live_panel = self.load_error.is_none()
            && self.view == View::Overview
            && mode == LayoutMode::Wide
            && !self.settings.sources.is_empty();
        let show_master_rail = self.load_error.is_none()
            && self.view == View::SourceSettings
            && screen_width >= MASTER_DETAIL_LEFT_WIDTH
            && !self.settings.sources.is_empty();
        let show_live_control_panel = self.load_error.is_none()
            && self.view == View::SourceSettings
            && screen_width >= MASTER_DETAIL_RIGHT_WIDTH
            && self.selected_source.is_some();

        if show_wide_live_panel {
            self.wide_live_panel(ctx, palette);
        }
        if show_master_rail {
            self.source_master_rail(ctx, palette);
        }
        if show_live_control_panel {
            self.source_live_control_panel(ctx, palette);
        }

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(palette.canvas_bg)
                    .inner_margin(Margin::symmetric(
                        if mode == LayoutMode::Narrow { 10 } else { 14 },
                        12,
                    )),
            )
            .show(ctx, |ui| {
                if self.load_error.is_some() {
                    ScrollArea::vertical().show(ui, |ui| self.load_error_page(ui));
                    return;
                }

                let narrow = mode == LayoutMode::Narrow;

                // Toolbar / Navigation bar or Breadcrumb bar
                ui.horizontal(|ui| {
                    ui.set_min_height(30.0);
                    ui.spacing_mut().item_spacing.x = 8.0;

                    if self.view == View::SourceSettings {
                        let source_title = self
                            .settings
                            .sources
                            .iter()
                            .find(|s| Some(s.id) == self.selected_source)
                            .map(|s| s.display_name().to_owned())
                            .unwrap_or_else(|| "Ljudkälla".to_owned());
                        if action_button(
                            ui,
                            Icon::Back,
                            "Översikt",
                            true,
                            palette.text_secondary,
                            ButtonVariant::Neutral,
                            palette,
                        )
                        .clicked()
                        {
                            self.navigate(View::Overview);
                        }
                        ui.label(
                            RichText::new("/")
                                .size(FONT_BODY)
                                .color(palette.text_disabled),
                        );
                        ui.add(
                            egui::Label::new(strong_text(
                                source_title,
                                FONT_TITLE_S,
                                palette.text_primary,
                            ))
                            .truncate(),
                        );
                    } else {
                        // Segmented view tabs
                        Frame::new()
                            .fill(palette.segmented_bg)
                            .stroke(Stroke::new(1.0, palette.pill_border))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(2))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if tab_button(
                                        ui,
                                        "Översikt",
                                        self.view == View::Overview,
                                        palette,
                                    )
                                    .clicked()
                                    {
                                        self.navigate(View::Overview);
                                    }
                                    if tab_button(
                                        ui,
                                        "Destinationer",
                                        self.view == View::Destinations,
                                        palette,
                                    )
                                    .clicked()
                                    {
                                        self.navigate(View::Destinations);
                                    }
                                });
                            });

                        let show_compact =
                            self.view == View::Overview && !self.settings.sources.is_empty();
                        if show_compact {
                            let mut compact = self
                                .settings
                                .compact_overview
                                .unwrap_or(self.settings.sources.len() >= 4);
                            if ui.checkbox(&mut compact, "Kompakt").changed() {
                                self.settings.compact_overview = Some(compact);
                                self.save_state.changed(Instant::now());
                            }
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_height(30.0);
                        ui.spacing_mut().item_spacing.x = 8.0;

                        if self.view == View::Overview {
                            let label = "Lägg till ljudkälla";
                            let use_icon_only = narrow || ui.available_width() < 235.0;
                            let add_source = if use_icon_only {
                                icon_button(
                                    ui,
                                    Icon::Plus,
                                    true,
                                    palette.text_primary,
                                    false,
                                    palette,
                                    "Lägg till ljudkälla (Ctrl+N)",
                                )
                            } else {
                                action_button(
                                    ui,
                                    Icon::Plus,
                                    label,
                                    true,
                                    palette.text_primary,
                                    ButtonVariant::Neutral,
                                    palette,
                                )
                                .on_hover_text("Lägg till ljudkälla (Ctrl+N)")
                            };
                            if add_source.clicked() {
                                self.add_source();
                            }
                        }

                        match self.save_state {
                            SaveState::Pending(_) => {
                                ui.weak(if narrow { "Sparar…" } else { "Sparar snart…" });
                            }
                            SaveState::Saved(at) if at.elapsed() < SAVED_FEEDBACK_DURATION => {
                                if narrow {
                                    status_pill(
                                        ui,
                                        "Sparat",
                                        palette.stream_pill_bg,
                                        palette.stream_active_border,
                                        palette.stream_accent,
                                    );
                                } else {
                                    saved_button(ui, "Sparat", palette);
                                }
                            }
                            _ => {}
                        }
                    });
                });

                if self.view != View::ListenerStats {
                    ui.add_space(10.0);
                }

                match self.view {
                    View::ListenerStats => {
                        centered_column(ui, FORM_MAX_WIDTH, |ui| {
                            listener_stats_page(
                                ui,
                                palette,
                                &self.settings.destinations,
                                &mut self.listener_stats,
                            );
                        });
                    }
                    View::Overview => {
                        if self.settings.sources.is_empty() {
                            empty_sources_view(ui, palette, || self.add_source());
                            return;
                        }
                        let mut open_settings_id = None;
                        let mut stream_toggle: Option<(u64, bool)> = None;
                        let mut record_toggle: Option<(u64, bool)> = None;
                        let compact = self
                            .settings
                            .compact_overview
                            .unwrap_or(self.settings.sources.len() >= 4);
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y =
                                    if narrow || compact { 10.0 } else { 14.0 };
                                let avail_w = ui.available_width();
                                let cols = overview_columns(
                                    avail_w,
                                    mode,
                                    compact,
                                    self.settings.sources.len(),
                                );
                                if cols > 1 {
                                    let spacing = if compact { 10.0 } else { 12.0 };
                                    let col_width = ((avail_w - spacing * (cols as f32 - 1.0))
                                        / cols as f32)
                                        .floor()
                                        .max(260.0);
                                    let card_narrow = narrow || (compact && col_width < 480.0);
                                    for row in self.settings.sources.chunks(cols) {
                                        ui.horizontal_top(|ui| {
                                            ui.spacing_mut().item_spacing.x = spacing;
                                            for source in row {
                                                ui.allocate_ui_with_layout(
                                                    Vec2::new(col_width, 0.0),
                                                    Layout::top_down(Align::Min),
                                                    |ui| {
                                                        ui.push_id(source.id, |ui| {
                                                            let actions = source_card(
                                                                ui,
                                                                &self.engine,
                                                                source,
                                                                &self.settings.destinations,
                                                                self.engine.streaming(source.id),
                                                                self.engine.recording(source.id),
                                                                true,
                                                                compact,
                                                                card_narrow,
                                                                palette,
                                                            );
                                                            if actions.open_settings {
                                                                open_settings_id = Some(source.id);
                                                            }
                                                            if let Some(start) = actions.set_stream {
                                                                stream_toggle = Some((source.id, start));
                                                            }
                                                            if let Some(start) = actions.set_record {
                                                                record_toggle = Some((source.id, start));
                                                            }
                                                        });
                                                    },
                                                );
                                            }
                                        });
                                    }
                                } else {
                                    let max_w = if narrow {
                                        ui.available_width()
                                    } else {
                                        OVERVIEW_MAX_WIDTH
                                    };
                                    centered_column(ui, max_w, |ui| {
                                        ui.spacing_mut().item_spacing.y =
                                            if narrow || compact { 10.0 } else { 14.0 };
                                        for source in &self.settings.sources {
                                            ui.push_id(source.id, |ui| {
                                                let actions = source_card(
                                                    ui,
                                                    &self.engine,
                                                    source,
                                                    &self.settings.destinations,
                                                    self.engine.streaming(source.id),
                                                    self.engine.recording(source.id),
                                                    true,
                                                    compact,
                                                    narrow,
                                                    palette,
                                                );
                                                if actions.open_settings {
                                                    open_settings_id = Some(source.id);
                                                }
                                                if let Some(start) = actions.set_stream {
                                                    stream_toggle = Some((source.id, start));
                                                }
                                                if let Some(start) = actions.set_record {
                                                    record_toggle = Some((source.id, start));
                                                }
                                            });
                                        }
                                    });
                                }
                            });

                        if let Some((id, start)) = stream_toggle {
                            self.set_source_stream(id, start);
                        }
                        if let Some((id, start)) = record_toggle {
                            self.set_source_record(id, start);
                        }
                        if let Some(id) = open_settings_id {
                            self.open_source_settings(id);
                        }
                    }
                    View::SourceSettings => {
                        let Some(index) = self
                            .settings
                            .sources
                            .iter()
                            .position(|source| Some(source.id) == self.selected_source)
                        else {
                            self.navigate(View::Overview);
                            return;
                        };
                        let source_id = self.settings.sources[index].id;
                        let streaming = self.engine.streaming(source_id);
                        let recording = self.engine.recording(source_id);
                        let active = streaming || recording;
                        let mut delete_this_source = false;
                        let mut stream_toggle: Option<bool> = None;
                        let mut record_toggle: Option<bool> = None;
                        let mut outcome = SourceSettingsOutcome::default();
                        let devices = self.engine.devices();
                        let streaming_dest_ids: HashSet<u64> = self
                            .settings
                            .destinations
                            .iter()
                            .filter(|d| self.destination_streaming(d.id))
                            .map(|d| d.id)
                            .collect();
                        let Settings {
                            destinations,
                            sources,
                            ..
                        } = &mut self.settings;
                        let source = &mut sources[index];
                        ScrollArea::vertical()
                            .id_salt(("source_settings", source_id))
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                centered_column(ui, FORM_MAX_WIDTH, |ui| {
                                    if !show_live_control_panel {
                                        let actions = source_card(
                                            ui,
                                            &self.engine,
                                            source,
                                            destinations,
                                            streaming,
                                            recording,
                                            false,
                                            false,
                                            narrow,
                                            palette,
                                        );
                                        if let Some(start) = actions.set_stream {
                                            stream_toggle = Some(start);
                                        }
                                        if let Some(start) = actions.set_record {
                                            record_toggle = Some(start);
                                        }
                                        ui.add_space(10.0);
                                    }

                                    outcome = ui
                                        .push_id(source.id, |ui| {
                                            source_settings(
                                                ui,
                                                source,
                                                destinations,
                                                &mut self.new_destination_draft,
                                                &mut self.inline_edit_destination,
                                                &mut self.connection_tester,
                                                &devices,
                                                streaming,
                                                recording,
                                                &streaming_dest_ids,
                                                narrow,
                                                palette,
                                            )
                                        })
                                        .inner;

                                    ui.add_space(10.0);
                                    card_frame(palette, palette.card_border).show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal_wrapped(|ui| {
                                            if with_disabled_reason(
                                                neutral_button_colored(
                                                    ui,
                                                    "Ta bort ljudkälla",
                                                    !active,
                                                    palette.danger,
                                                    palette,
                                                ),
                                                !active,
                                                Some("Stoppa sändning och inspelning innan du tar bort ljudkällan"),
                                            )
                                            .clicked()
                                            {
                                                delete_this_source = true;
                                            }
                                            if !active {
                                                ui.label(
                                                    RichText::new("Du kan ångra direkt efter borttagning.")
                                                        .size(FONT_CAPTION)
                                                        .color(palette.text_secondary),
                                                );
                                            }
                                        });
                                    });
                                });
                            });
                        if outcome.refresh_devices {
                            self.engine.refresh_devices();
                        }
                        if let Some(start) = stream_toggle {
                            self.set_source_stream(source_id, start);
                        }
                        if let Some(start) = record_toggle {
                            self.set_source_record(source_id, start);
                        }
                        if delete_this_source {
                            self.delete_source(index);
                        } else if outcome.changed {
                            self.save_state.changed(Instant::now());
                            self.engine.update(&self.settings.sources);
                        }
                    }
                    View::Destinations => {
                        if self.settings.destinations.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(60.0);
                                ui.label(strong_text(
                                    "Lägg till en destination",
                                    FONT_TITLE_L,
                                    palette.text_primary,
                                ));
                                ui.add_space(SPACE_S);
                                ui.label(
                                    RichText::new(
                                        "Välj vart ljudet ska skickas när du sänder.",
                                    )
                                    .color(palette.text_secondary),
                                );
                                crate::guide::tip(ui, "Be den som sköter sändningen om adress och lösenord.");
                                ui.add_space(10.0);
                                if action_button(
                                    ui,
                                    Icon::Plus,
                                    "Lägg till destination",
                                    true,
                                    palette.tab_active_fg,
                                    ButtonVariant::Primary,
                                    palette,
                                )
                                .clicked()
                                {
                                    self.add_destination();
                                }
                            });
                            return;
                        }
                        let mut changed = false;
                        let mut remove_id: Option<u64> = None;
                        let mut add_new = false;
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                centered_column(ui, FORM_MAX_WIDTH, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(strong_text(
                                            "Sparade destinationer",
                                            FONT_TITLE_M,
                                            palette.text_primary,
                                        ));
                                        crate::guide::tip(ui, "Här sparar du vart ljudet ska skickas. Be den som sköter sändningen om adress och lösenord.");
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if action_button(
                                                    ui,
                                                    Icon::Plus,
                                                    if narrow { "Lägg till" } else { "Lägg till destination" },
                                                    true,
                                                    palette.text_primary,
                                                    ButtonVariant::Neutral,
                                                    palette,
                                                )
                                                .clicked()
                                                {
                                                    add_new = true;
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(SPACE_M);
                                    for index in 0..self.settings.destinations.len() {
                                        let dest_id = self.settings.destinations[index].id;
                                        let used_by: Vec<String> = self
                                            .settings
                                            .sources
                                            .iter()
                                            .filter(|source| source.destination_ids.contains(&dest_id))
                                            .map(|source| source.display_name().to_owned())
                                            .collect();
                                        let in_use_streaming = self.destination_streaming(dest_id);
                                        let expanded = self.expanded_destinations.contains(&dest_id);
                                        let card_border = if expanded {
                                            palette.stream_active_border
                                        } else {
                                            palette.card_border
                                        };
                                        ui.push_id(dest_id, |ui| {
                                            card_frame(palette, card_border).show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                let dest = &mut self.settings.destinations[index];
                                                let mut toggle_expand = false;
                                                ui.horizontal_wrapped(|ui| {
                                                    if icon_button(
                                                        ui,
                                                        if expanded {
                                                            Icon::ChevronDown
                                                        } else {
                                                            Icon::ChevronRight
                                                        },
                                                        true,
                                                        palette.text_secondary,
                                                        false,
                                                        palette,
                                                        if expanded {
                                                            "Fäll ihop"
                                                        } else {
                                                            "Visa och redigera"
                                                        },
                                                    )
                                                    .clicked()
                                                    {
                                                        toggle_expand = true;
                                                    }
                                                    ui.label(strong_text(
                                                        dest.display_name(),
                                                        FONT_TITLE_S,
                                                        palette.text_primary,
                                                    ));
                                                    status_pill(
                                                        ui,
                                                        if used_by.is_empty() {
                                                            "Används inte".to_owned()
                                                        } else {
                                                            format!(
                                                                "Används av: {}",
                                                                used_by.join(", ")
                                                            )
                                                        },
                                                        palette.pill_bg,
                                                        palette.pill_border,
                                                        palette.text_secondary,
                                                    );
                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Center),
                                                        |ui| {
                                                            if compact_button(
                                                                ui,
                                                                if expanded { "Klar" } else { "Redigera" },
                                                                true,
                                                                None,
                                                                palette,
                                                            )
                                                            .clicked()
                                                            {
                                                                toggle_expand = true;
                                                            }
                                                        },
                                                    );
                                                });

                                                if !expanded {
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        RichText::new(destination_summary(dest))
                                                            .monospace()
                                                            .size(FONT_CAPTION)
                                                            .color(palette.text_secondary),
                                                    );
                                                } else {
                                                    if in_use_streaming {
                                                        ui.add_space(4.0);
                                                        lock_badge(
                                                            ui,
                                                            "Låst under pågående sändning",
                                                            palette,
                                                        );
                                                    }
                                                    ui.add_space(SPACE_M);
                                                    ui.add_enabled_ui(!in_use_streaming, |ui| {
                                                        let dest_changed = destination_settings(
                                                            ui,
                                                            dest,
                                                            narrow,
                                                            palette,
                                                        );
                                                        if dest_changed {
                                                            self.connection_tester.clear(dest_id);
                                                            changed = true;
                                                        }
                                                    });
                                                    ui.add_space(SPACE_M);
                                                    let (sep, _) = ui.allocate_exact_size(
                                                        Vec2::new(ui.available_width(), 1.0),
                                                        Sense::hover(),
                                                    );
                                                    ui.painter().rect_filled(
                                                        sep,
                                                        CornerRadius::ZERO,
                                                        palette.card_divider,
                                                    );
                                                    ui.add_space(SPACE_M);
                                                    ui.horizontal_wrapped(|ui| {
                                                        connection_test_row(
                                                            ui,
                                                            dest,
                                                            &mut self.connection_tester,
                                                            palette,
                                                        );
                                                        ui.with_layout(
                                                            Layout::right_to_left(Align::Center),
                                                            |ui| {
                                                                if with_disabled_reason(
                                                                    compact_button(
                                                                        ui,
                                                                        "Ta bort destination",
                                                                        !in_use_streaming,
                                                                        Some(palette.danger),
                                                                        palette,
                                                                    ),
                                                                    !in_use_streaming,
                                                                    Some("Stoppa pågående sändning innan du tar bort destinationen"),
                                                                )
                                                                .clicked()
                                                                {
                                                                    remove_id = Some(dest_id);
                                                                }
                                                            },
                                                        );
                                                    });
                                                }

                                                if toggle_expand {
                                                    if expanded {
                                                        self.expanded_destinations.remove(&dest_id);
                                                    } else {
                                                        self.expanded_destinations.insert(dest_id);
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(10.0);
                                    }
                                });
                            });
                        if add_new {
                            self.add_destination();
                        }
                        if let Some(dest_id) = remove_id {
                            self.delete_destination(dest_id);
                        } else if changed {
                            self.save_state.changed(Instant::now());
                            self.engine.update(&self.settings.sources);
                        }
                    }
                }
            });

        let guard_close =
            closing && guard_active_close(ctx, self.close_confirmed, self.has_active_outputs());
        if guard_close {
            self.confirm_stop = Some(StopConfirm::Close {
                discard_changes: false,
            });
        }
        if self.confirm_stop.is_some() {
            self.stop_confirm_modal(ctx, palette);
        } else if let Some(recording) = self.saved_recordings.front_mut() {
            if recording.show(ctx) {
                self.saved_recordings.pop_front();
            }
        } else {
            self.undo_toast_overlay(ctx, palette);
        }

        let closing = closing && !guard_close;
        let now = Instant::now();
        if self.save_state.due(now) || (closing && self.save_state.dirty()) {
            self.save();
            if closing && self.save_state.dirty() {
                self.hidden_settings_error = None;
                self.close_confirmed = false;
                self.close_blocked = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        let mut repaint_after = self.repaint_interval();
        if let Some(toast) = &self.undo_toast {
            if let Some(remaining) = UNDO_TOAST_DURATION.checked_sub(toast.created_at.elapsed()) {
                repaint_after = repaint_after.min(remaining);
            } else {
                self.undo_toast = None;
            }
        }
        match self.save_state {
            SaveState::Pending(deadline) => {
                repaint_after = repaint_after.min(deadline.saturating_duration_since(now));
            }
            SaveState::Saved(at) => {
                if let Some(remaining) = SAVED_FEEDBACK_DURATION.checked_sub(at.elapsed()) {
                    repaint_after = repaint_after.min(remaining);
                } else {
                    self.save_state = SaveState::Idle;
                }
            }
            _ => {}
        }
        ctx.request_repaint_after(repaint_after);
    }
}

fn settings_error_detail(ui: &mut EguiUi, error: &str, hidden: &mut Option<String>) {
    if hidden.as_deref() != Some(error) {
        *hidden = None;
        if feedback::message(ui, error) {
            *hidden = Some(error.to_owned());
        }
    } else if ui.small_button("Visa felmeddelandet").clicked() {
        *hidden = None;
    }
}

fn guard_active_close(ctx: &Context, confirmed: bool, active: bool) -> bool {
    if confirmed || !active {
        return false;
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    true
}

fn listener_stats_page(
    ui: &mut EguiUi,
    palette: Palette,
    destinations: &[Destination],
    stats: &mut ListenerStats,
) {
    let targets: Vec<_> = destinations.iter().map(StatsTarget::parse).collect();
    let refresh = ui
        .horizontal(|ui| {
            ui.label(strong_text("Lyssnarstatistik", 18.0, palette.text_primary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let refreshing = stats.refreshing();
                let clicked = text_button(
                    ui,
                    "Uppdatera",
                    targets.iter().any(Result::is_ok) && !refreshing,
                    ButtonVariant::Neutral,
                    palette,
                )
                .clicked();
                if refreshing {
                    ui.spinner();
                }
                clicked
            })
            .inner
        })
        .inner;
    stats.update(
        ui.ctx(),
        targets.iter().filter_map(|target| target.as_ref().ok()),
        refresh,
    );

    if destinations.is_empty() {
        ui.add_space(SPACE_M);
        ui.weak(
            "Lägg till en Icecast-destination under Destinationer för att se lyssnarstatistik.",
        );
        return;
    }

    ui.add_space(SPACE_M);
    ScrollArea::vertical()
        .id_salt("listener_stats")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            for (destination, target) in destinations.iter().zip(&targets) {
                card_frame(palette, palette.card_border).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(strong_text(
                        destination.display_name(),
                        15.0,
                        palette.text_primary,
                    ));
                    let issue_id = egui::Id::new(("stats-error", destination.id));
                    let target = match target {
                        Ok(target) => target,
                        Err(error) => {
                            ui.weak("Statistik saknas");
                            feedback::condition(ui, issue_id, Some(error));
                            return;
                        }
                    };
                    ui.weak(format!("{}{}", target.server, target.mount));
                    let status = stats.status(target);
                    match &status {
                        MountStatus::Loading => {
                            ui.weak("Hämtar statistik…");
                        }
                        MountStatus::Unavailable(_) => {
                            ui.weak("Statistik saknas");
                        }
                        MountStatus::Absent => {
                            ui.weak("Monteringspunkten visas inte i serverns statistik.");
                        }
                        MountStatus::Listed(counts) => {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 28.0;
                                let metrics = [
                                    ("Lyssnare nu", counts.current),
                                    ("Lyssnartopp", counts.peak),
                                ];
                                for (label, count) in metrics {
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.weak(label);
                                        let value = count.map_or_else(
                                            || "—".to_owned(),
                                            |count| count.to_string(),
                                        );
                                        ui.label(strong_text(value, 24.0, palette.text_primary));
                                    });
                                }
                            });
                        }
                    }
                    let reason = match status {
                        MountStatus::Unavailable(error) => Some(error),
                        _ => None,
                    };
                    feedback::condition(ui, issue_id, reason);
                });
            }
        });
}

fn configure_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    let regular_candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ]
    };

    let bold_candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Windows\Fonts\seguisb.ttf",
            r"C:\Windows\Fonts\segoeuib.ttf",
            r"C:\Windows\Fonts\arialbd.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ]
    };

    if let Some(bytes) = load_first_font(regular_candidates) {
        fonts.font_data.insert(
            "ui-regular".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "ui-regular".to_owned());
    }

    let mut strong_family = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    if let Some(bytes) = load_first_font(bold_candidates) {
        fonts.font_data.insert(
            "ui-strong".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        strong_family.insert(0, "ui-strong".to_owned());
    }

    fonts
        .families
        .insert(FontFamily::Name(STRONG_FONT.into()), strong_family);

    ctx.set_fonts(fonts);
}

fn load_first_font(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|path| std::fs::read(path).ok())
}

fn strong_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text)
        .font(FontId::new(size, FontFamily::Name(STRONG_FONT.into())))
        .color(color)
}

fn build_visuals(dark: bool) -> egui::Visuals {
    let palette = Palette::for_theme(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = palette.canvas_bg;
    visuals.window_fill = palette.card_bg;
    visuals.faint_bg_color = palette.pill_bg;
    visuals.extreme_bg_color = palette.meter_track;
    visuals.override_text_color = None;

    visuals.selection.bg_fill = palette.tab_active_bg;
    visuals.selection.stroke = Stroke::new(1.0, palette.tab_active_fg);
    visuals.hyperlink_color = palette.stream_accent;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;

    visuals.widgets.noninteractive.bg_fill = palette.card_bg;
    visuals.widgets.noninteractive.weak_bg_fill = palette.pill_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.card_border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(7);

    visuals.widgets.inactive.bg_fill = palette.btn_bg;
    visuals.widgets.inactive.weak_bg_fill = palette.btn_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.btn_border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(7);

    visuals.widgets.hovered.bg_fill = palette.btn_bg_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.btn_bg_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.btn_border_hover);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(7);

    visuals.widgets.active.bg_fill = palette.btn_bg_active;
    visuals.widgets.active.weak_bg_fill = palette.btn_bg_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.stream_accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.active.corner_radius = CornerRadius::same(7);

    visuals.widgets.open.bg_fill = palette.btn_bg_hover;
    visuals.widgets.open.weak_bg_fill = palette.btn_bg_hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.stream_accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.open.corner_radius = CornerRadius::same(7);

    visuals.window_stroke = Stroke::new(1.0, palette.card_border);
    visuals.window_corner_radius = CornerRadius::same(10);

    visuals
}

fn card_frame(palette: Palette, border: Color32) -> Frame {
    Frame::new()
        .fill(palette.card_bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 12))
        .shadow(palette.card_shadow)
}

fn load_app_icon_texture(ctx: &Context) -> Option<TextureHandle> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")).ok()?;
    let image =
        ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
    Some(ctx.load_texture("app-icon", image, TextureOptions::LINEAR))
}

fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;

    match icon {
        Icon::Help => {
            painter.circle_stroke(c, r * 0.93, Stroke::new(1.3, color));
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "?",
                FontId::proportional(r * 1.55),
                color,
            );
        }
        Icon::ListenerStats => {
            for (x, height) in [(-0.64, 0.65), (0.0, 1.14), (0.64, 1.60)] {
                let bottom = c + Vec2::new(x * r, r * 0.8);
                painter.line_segment(
                    [bottom, bottom - Vec2::new(0.0, r * height)],
                    Stroke::new(2.0, color),
                );
            }
        }
        Icon::Broadcast => {
            painter.circle_filled(c, (r * 0.22).max(1.8), color);
            paint_arc(painter, c, r * 0.54, -0.68, 0.68, Stroke::new(1.6, color));
            paint_arc(
                painter,
                c,
                r * 0.54,
                PI - 0.68,
                PI + 0.68,
                Stroke::new(1.6, color),
            );
            paint_arc(painter, c, r * 0.88, -0.78, 0.78, Stroke::new(1.6, color));
            paint_arc(
                painter,
                c,
                r * 0.88,
                PI - 0.78,
                PI + 0.78,
                Stroke::new(1.6, color),
            );
        }
        Icon::BroadcastStop => {
            let faded = color.gamma_multiply(0.65);
            painter.circle_filled(c, (r * 0.20).max(1.6), faded);
            paint_arc(painter, c, r * 0.54, -0.65, 0.65, Stroke::new(1.5, faded));
            paint_arc(
                painter,
                c,
                r * 0.54,
                PI - 0.65,
                PI + 0.65,
                Stroke::new(1.5, faded),
            );
            paint_arc(painter, c, r * 0.86, -0.75, 0.75, Stroke::new(1.5, faded));
            paint_arc(
                painter,
                c,
                r * 0.86,
                PI - 0.75,
                PI + 0.75,
                Stroke::new(1.5, faded),
            );
            // Crisp diagonal stop slash
            let d = r * 0.78;
            painter.line_segment(
                [Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)],
                Stroke::new(2.0, color),
            );
        }
        Icon::Record => {
            painter.circle_stroke(c, r * 0.82, Stroke::new(1.6, color));
            painter.circle_filled(c, r * 0.44, color);
        }
        Icon::RecordStop => {
            painter.circle_stroke(c, r * 0.82, Stroke::new(1.6, color));
            let sq = Rect::from_center_size(c, Vec2::splat(r * 0.74));
            painter.rect_filled(sq, CornerRadius::same(1), color);
        }
        Icon::Stop => {
            let sq = Rect::from_center_size(c, Vec2::splat(r * 1.18));
            painter.rect_filled(sq, CornerRadius::same(2), color);
        }
        Icon::Folder => {
            let outline = [
                (-0.9, -0.65),
                (-0.25, -0.65),
                (0.0, -0.35),
                (0.9, -0.35),
                (0.9, 0.65),
                (-0.9, 0.65),
            ];
            let points = outline
                .into_iter()
                .map(|(x, y)| c + Vec2::new(x * r, y * r))
                .collect();
            painter.add(Shape::closed_line(points, Stroke::new(1.5, color)));
        }
        Icon::Settings => {
            // 3 horizontal slider tracks with staggered knobs
            let stroke = Stroke::new(1.4, color);
            let rows = [(-0.56, -0.25), (0.0, 0.30), (0.56, -0.10)];
            for (y_off, knob_x) in rows {
                let y = c.y + r * y_off;
                painter.line_segment(
                    [Pos2::new(c.x - r * 0.82, y), Pos2::new(c.x + r * 0.82, y)],
                    stroke,
                );
                painter.circle_filled(Pos2::new(c.x + r * knob_x, y), (r * 0.24).max(1.8), color);
            }
        }
        Icon::Plus => {
            let d = r * 0.72;
            let stroke = Stroke::new(1.8, color);
            painter.line_segment([Pos2::new(c.x - d, c.y), Pos2::new(c.x + d, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x, c.y - d), Pos2::new(c.x, c.y + d)], stroke);
        }

        Icon::Check => {
            let stroke = Stroke::new(1.8, color);
            let points = vec![
                Pos2::new(c.x - r * 0.65, c.y + r * 0.05),
                Pos2::new(c.x - r * 0.15, c.y + r * 0.55),
                Pos2::new(c.x + r * 0.72, c.y - r * 0.48),
            ];
            painter.add(Shape::line(points, stroke));
        }
        Icon::Theme => {
            // Contrast half-moon/sun circle
            painter.circle_stroke(c, r * 0.82, Stroke::new(1.5, color));
            let steps = 12;
            let mut half: Vec<Pos2> = (0..=steps)
                .map(|i| {
                    let angle = -PI * 0.5 + PI * (i as f32 / steps as f32);
                    Pos2::new(
                        c.x + (r * 0.72) * angle.cos(),
                        c.y + (r * 0.72) * angle.sin(),
                    )
                })
                .collect();
            half.push(Pos2::new(c.x, c.y - r * 0.72));
            painter.add(Shape::convex_polygon(half, color, Stroke::NONE));
        }
        Icon::More => {
            let dot_r = (r * 0.18).max(1.4);
            for dx in [-0.58, 0.0, 0.58] {
                painter.circle_filled(Pos2::new(c.x + dx * r, c.y), dot_r, color);
            }
        }
        Icon::Back => {
            let stroke = Stroke::new(1.7, color);
            painter.line_segment(
                [
                    Pos2::new(c.x - r * 0.65, c.y),
                    Pos2::new(c.x + r * 0.72, c.y),
                ],
                stroke,
            );
            painter.add(Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.12, c.y - r * 0.52),
                    Pos2::new(c.x - r * 0.65, c.y),
                    Pos2::new(c.x - r * 0.12, c.y + r * 0.52),
                ],
                stroke,
            ));
        }
        Icon::Close => {
            let d = r * 0.58;
            let stroke = Stroke::new(1.7, color);
            painter.line_segment(
                [Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + d, c.y - d), Pos2::new(c.x - d, c.y + d)],
                stroke,
            );
        }
        Icon::Lock => {
            let body = Rect::from_min_max(
                Pos2::new(c.x - r * 0.64, c.y - r * 0.08),
                Pos2::new(c.x + r * 0.64, c.y + r * 0.78),
            );
            painter.rect_stroke(
                body,
                CornerRadius::same(2),
                Stroke::new(1.5, color),
                egui::StrokeKind::Inside,
            );
            paint_arc(
                painter,
                Pos2::new(c.x, c.y - r * 0.18),
                r * 0.40,
                PI,
                PI * 2.0,
                Stroke::new(1.5, color),
            );
        }
        Icon::Refresh => {
            let stroke = Stroke::new(1.6, color);
            paint_arc(painter, c, r * 0.68, -0.3, PI * 1.45, stroke);
            let tip = Pos2::new(
                c.x + r * 0.68 * (-0.3_f32).cos(),
                c.y + r * 0.68 * (-0.3_f32).sin(),
            );
            painter.add(Shape::line(
                vec![
                    Pos2::new(tip.x - r * 0.34, tip.y - r * 0.10),
                    tip,
                    Pos2::new(tip.x + r * 0.05, tip.y - r * 0.38),
                ],
                stroke,
            ));
        }
        Icon::ChevronRight => {
            let stroke = Stroke::new(1.7, color);
            painter.add(Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.28, c.y - r * 0.52),
                    Pos2::new(c.x + r * 0.30, c.y),
                    Pos2::new(c.x - r * 0.28, c.y + r * 0.52),
                ],
                stroke,
            ));
        }
        Icon::ChevronDown => {
            let stroke = Stroke::new(1.7, color);
            painter.add(Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.52, c.y - r * 0.26),
                    Pos2::new(c.x, c.y + r * 0.30),
                    Pos2::new(c.x + r * 0.52, c.y - r * 0.26),
                ],
                stroke,
            ));
        }
        Icon::Eye => {
            let stroke = Stroke::new(1.5, color);
            paint_arc(
                painter,
                Pos2::new(c.x, c.y + r * 0.55),
                r * 0.95,
                -PI * 0.78,
                -PI * 0.22,
                stroke,
            );
            paint_arc(
                painter,
                Pos2::new(c.x, c.y - r * 0.55),
                r * 0.95,
                PI * 0.22,
                PI * 0.78,
                stroke,
            );
            painter.circle_filled(c, (r * 0.24).max(1.6), color);
        }
        Icon::EyeOff => {
            let stroke = Stroke::new(1.4, color);
            paint_arc(
                painter,
                Pos2::new(c.x, c.y + r * 0.55),
                r * 0.95,
                -PI * 0.78,
                -PI * 0.22,
                stroke,
            );
            paint_arc(
                painter,
                Pos2::new(c.x, c.y - r * 0.55),
                r * 0.95,
                PI * 0.22,
                PI * 0.78,
                stroke,
            );
            let d = r * 0.70;
            painter.line_segment(
                [Pos2::new(c.x - d, c.y + d), Pos2::new(c.x + d, c.y - d)],
                Stroke::new(1.7, color),
            );
        }
        Icon::Undo => {
            let stroke = Stroke::new(1.6, color);
            paint_arc(
                painter,
                Pos2::new(c.x + r * 0.12, c.y + r * 0.14),
                r * 0.56,
                -PI * 0.9,
                PI * 0.25,
                stroke,
            );
            let tip = Pos2::new(c.x - r * 0.44, c.y - r * 0.08);
            painter.add(Shape::line(
                vec![
                    Pos2::new(tip.x + r * 0.05, tip.y - r * 0.42),
                    tip,
                    Pos2::new(tip.x + r * 0.42, tip.y + r * 0.04),
                ],
                stroke,
            ));
        }
    }
}

fn apply_theme_preference(ctx: &Context, pref: ThemePreference) {
    let egui_pref = match pref {
        ThemePreference::System => egui::ThemePreference::System,
        ThemePreference::Light => egui::ThemePreference::Light,
        ThemePreference::Dark => egui::ThemePreference::Dark,
    };
    ctx.set_theme(egui_pref);
}

fn centered_column(ui: &mut EguiUi, max_width: f32, add_contents: impl FnOnce(&mut EguiUi)) {
    let available = ui.available_width();
    if available <= max_width + 2.0 {
        add_contents(ui);
    } else {
        let avail_rect = ui.available_rect_before_wrap();
        let side = (available - max_width) * 0.5;
        let centered_rect = Rect::from_min_max(
            Pos2::new(avail_rect.left() + side, avail_rect.top()),
            Pos2::new(avail_rect.left() + side + max_width, avail_rect.bottom()),
        );
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(centered_rect)
                .layout(Layout::top_down(Align::Min)),
            add_contents,
        );
    }
}

fn overview_columns(
    avail_width: f32,
    mode: LayoutMode,
    compact: bool,
    source_count: usize,
) -> usize {
    if source_count <= 1 {
        return 1;
    }
    if compact {
        let max_cols = if avail_width >= 1000.0 {
            3
        } else if avail_width >= 660.0 {
            2
        } else {
            1
        };
        max_cols.min(source_count)
    } else if mode == LayoutMode::Wide {
        2.min(source_count)
    } else {
        1
    }
}

fn format_hms(duration: Duration) -> String {
    let secs = duration.as_secs();
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

fn format_dbfs(signal: f32) -> String {
    if signal.is_finite() && signal >= 0.001 {
        format!("{:.1} dBFS", 20.0 * signal.log10())
    } else {
        "−∞ dBFS".to_owned()
    }
}

fn lock_badge(ui: &mut EguiUi, text: &str, palette: Palette) {
    Frame::new()
        .fill(palette.warn_pill_bg)
        .stroke(Stroke::new(1.0, palette.warn_pill_border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                paint_icon(ui.painter(), icon_rect, Icon::Lock, palette.warning);
                ui.label(
                    RichText::new(text)
                        .size(FONT_CAPTION)
                        .color(palette.warning),
                );
            });
        });
}

fn dot_pill(
    ui: &mut EguiUi,
    text: &str,
    dot_color: Color32,
    bg: Color32,
    border: Color32,
    fg: Color32,
) {
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(7.0, 14.0), Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 3.2, dot_color);
                ui.label(RichText::new(text).size(FONT_CAPTION).color(fg));
            });
        });
}

fn wrapping_dot_pill(
    ui: &mut EguiUi,
    text: &str,
    dot_color: Color32,
    bg: Color32,
    border: Color32,
    fg: Color32,
) {
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(7.0, 14.0), Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 3.2, dot_color);
                ui.add(egui::Label::new(RichText::new(text).size(FONT_CAPTION).color(fg)).wrap());
            });
        });
}

fn dismissible_dot_pill(
    ui: &mut EguiUi,
    id: egui::Id,
    text: Option<&str>,
    dot_color: Color32,
    bg: Color32,
    border: Color32,
    fg: Color32,
) {
    feedback::show_if_needed(ui, id, text, |ui, message| {
        Frame::new()
            .fill(bg)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        let dismissed = ui
                            .add(
                                egui::Button::new(RichText::new("Dölj").size(11.0).color(fg))
                                    .frame(false),
                            )
                            .on_hover_text("Dölj meddelandet")
                            .clicked();
                        ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::new(7.0, 14.0), Sense::hover());
                            ui.painter()
                                .circle_filled(dot_rect.center(), 3.2, dot_color);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(message).size(FONT_CAPTION).color(fg),
                                )
                                .wrap(),
                            );
                        });
                        dismissed
                    })
                    .inner
                })
                .inner
            })
            .inner
    });
}

fn source_status_notices(
    ui: &mut EguiUi,
    engine: &Engine,
    source: &Source,
    streaming: bool,
    recording: bool,
    live: &crate::engine::LiveStatus,
    palette: Palette,
) {
    let source_id = egui::Id::new(("source-feedback", source.id));
    let input_hint = (!live.present).then(|| {
        let consequence = match (streaming, recording) {
            (true, true) => "Sändningen och inspelningen får tyst ljud. ",
            (true, false) => "Sändningen får tyst ljud. ",
            (false, true) => "Inspelningen får tyst ljud. ",
            (false, false) => "",
        };
        format!("{consequence}{}", engine.input_hint(source))
    });
    let has_other_targets = live
        .targets
        .iter()
        .any(|t| !t.sending || matches!(t.detail, OutputStatus::Problem(_)) || t.audio_gap);
    let has_record_notice = !live.record_detail.is_empty() || live.record_audio_gap;

    if input_hint.is_some() || has_other_targets || has_record_notice {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;

            dismissible_dot_pill(
                ui,
                source_id.with(("input", &source.device)),
                input_hint.as_deref(),
                palette.warning,
                palette.warn_pill_bg,
                palette.warn_pill_border,
                palette.warning,
            );

            for (index, target) in live.targets.iter().enumerate() {
                let id = source_id.with(("stream", live.stream_attempt, index));
                let problem = match &target.detail {
                    OutputStatus::Problem(reason) => Some(format!("{} · {reason}", target.name)),
                    OutputStatus::Progress(status) => {
                        if !target.sending {
                            wrapping_dot_pill(
                                ui,
                                &format!("{} · {status}", target.name),
                                palette.warning,
                                palette.warn_pill_bg,
                                palette.warn_pill_border,
                                palette.warning,
                            );
                        }
                        None
                    }
                };
                dismissible_dot_pill(
                    ui,
                    id,
                    problem.as_deref(),
                    palette.warning,
                    palette.warn_pill_bg,
                    palette.warn_pill_border,
                    palette.warning,
                );
                let gap = target.audio_gap.then(|| {
                    format!(
                        "{} · Delar av ljudet har fallit bort under sändningen.",
                        target.name
                    )
                });
                dismissible_dot_pill(
                    ui,
                    id.with("audio-gap"),
                    gap.as_deref(),
                    palette.warning,
                    palette.warn_pill_bg,
                    palette.warn_pill_border,
                    palette.warning,
                );
            }

            if !live.record_detail.is_empty() && !recording {
                let saved = live.record_detail.starts_with("Sparad · ");
                let (dot, bg, border, fg) = if saved {
                    (
                        palette.stream_accent,
                        palette.stream_pill_bg,
                        palette.stream_active_border,
                        palette.stream_accent,
                    )
                } else {
                    (
                        palette.warning,
                        palette.warn_pill_bg,
                        palette.warn_pill_border,
                        palette.warning,
                    )
                };
                wrapping_dot_pill(
                    ui,
                    &format!("Inspelning · {}", live.record_detail),
                    dot,
                    bg,
                    border,
                    fg,
                );
            }
            dismissible_dot_pill(
                ui,
                source_id.with(("record-gap", live.record_attempt)),
                live.record_audio_gap
                    .then_some("Inspelning · Delar av ljudet har fallit bort i inspelningen."),
                palette.warning,
                palette.warn_pill_bg,
                palette.warn_pill_border,
                palette.warning,
            );
        });
    } else {
        // Clear any previously dismissed state once conditions recover.
        for index in 0..live.targets.len() {
            let id = source_id.with(("stream", live.stream_attempt, index));
            feedback::condition(ui, id, None);
            feedback::condition(ui, id.with("audio-gap"), None);
        }
        feedback::condition(ui, source_id.with(("input", &source.device)), None);
        feedback::condition(
            ui,
            source_id.with(("record-gap", live.record_attempt)),
            None,
        );
    }
}

fn segmented_choice(
    ui: &mut EguiUi,
    id_salt: &str,
    current: &mut String,
    values: &[&str],
    palette: Palette,
) -> bool {
    let mut changed = false;
    ui.push_id(id_salt, |ui| {
        Frame::new()
            .fill(palette.segmented_bg)
            .stroke(Stroke::new(1.0, palette.pill_border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for &value in values {
                        let selected = current.eq_ignore_ascii_case(value);
                        if tab_button(ui, value, selected, palette).clicked() && !selected {
                            *current = value.to_owned();
                            changed = true;
                        }
                    }
                });
            });
    });
    changed
}

fn segmented_u32(
    ui: &mut EguiUi,
    id_salt: &str,
    current: &mut u32,
    options: &[(u32, &str)],
    palette: Palette,
) -> bool {
    let mut changed = false;
    ui.push_id(id_salt, |ui| {
        Frame::new()
            .fill(palette.segmented_bg)
            .stroke(Stroke::new(1.0, palette.pill_border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for &(value, label) in options {
                        let selected = *current == value;
                        if tab_button(ui, label, selected, palette).clicked() && !selected {
                            *current = value;
                            changed = true;
                        }
                    }
                });
            });
    });
    changed
}

fn segmented_bool(
    ui: &mut EguiUi,
    id_salt: &str,
    value: &mut bool,
    false_label: &str,
    true_label: &str,
    true_enabled: bool,
    palette: Palette,
) -> bool {
    let mut changed = false;
    ui.push_id(id_salt, |ui| {
        Frame::new()
            .fill(palette.segmented_bg)
            .stroke(Stroke::new(1.0, palette.pill_border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if tab_button(ui, false_label, !*value, palette).clicked() && *value {
                        *value = false;
                        changed = true;
                    }
                    ui.add_enabled_ui(true_enabled, |ui| {
                        if tab_button(ui, true_label, *value, palette).clicked() && !*value {
                            *value = true;
                            changed = true;
                        }
                    });
                });
            });
    });
    changed
}

fn full_width_action_button(
    ui: &mut EguiUi,
    icon: impl Into<Option<Icon>>,
    label: &str,
    enabled: bool,
    icon_color: Color32,
    variant: ButtonVariant,
    palette: Palette,
) -> egui::Response {
    render_button_sized(
        ui,
        ButtonSpec {
            icon: icon.into().map(|icon| (icon, icon_color)),
            label,
            enabled,
            variant,
            text_color_override: None,
            height: 32.0,
            font_size: FONT_BODY,
            pad_x: 10.0,
        },
        Some(ui.available_width()),
        palette,
    )
}

fn paint_sparkline(ui: &mut EguiUi, samples: &VecDeque<u64>, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
    if samples.len() < 2 || rect.width() <= 4.0 {
        return;
    }
    let min_v = *samples.iter().min().unwrap_or(&0) as f32;
    let max_v = *samples.iter().max().unwrap_or(&1) as f32;
    let span = (max_v - min_v).max(1.0);
    let inner = rect.shrink2(Vec2::new(2.0, 3.0));
    let n = (samples.len() - 1) as f32;
    let points: Vec<Pos2> = samples
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let x = inner.left() + inner.width() * (i as f32 / n);
            let norm = (val as f32 - min_v) / span;
            let y = inner.bottom() - inner.height() * norm;
            Pos2::new(x, y)
        })
        .collect();
    ui.painter()
        .add(Shape::line(points, Stroke::new(1.8, palette.stream_accent)));
}

fn vertical_stereo_meter(ui: &mut EguiUi, levels: &[MeterLevel], palette: Palette) {
    let total_height = 176.0;
    let bar_w = 22.0;
    let scale_w = 34.0;
    let stereo = levels.len() == 2;
    let widget_w = if stereo {
        bar_w * 2.0 + scale_w + 16.0
    } else {
        bar_w + scale_w + 12.0
    };
    let left_pad = ((ui.available_width() - widget_w) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(left_pad);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(widget_w, total_height + 16.0), Sense::hover());
        let painter = ui.painter();
        let top = rect.top() + 14.0;
        let bar_h = total_height;

        let draw_bar = |bar_rect: Rect, level: MeterLevel, label: &str| {
            painter.text(
                Pos2::new(bar_rect.center().x, rect.top() + 6.0),
                egui::Align2::CENTER_CENTER,
                label,
                FontId::proportional(11.0),
                palette.text_secondary,
            );
            painter.rect_filled(bar_rect, CornerRadius::same(4), palette.meter_track);
            let inner = bar_rect.shrink(1.0);
            let frac = meter_fraction(level.signal);
            if frac > 0.002 && inner.height() > 0.0 {
                let segments = 24usize;
                let seg_h = inner.height() / segments as f32;
                let lit = (frac * segments as f32).ceil() as usize;
                for i in 0..lit.min(segments) {
                    let f = (i as f32 + 0.5) / segments as f32;
                    let color = meter_gradient_color(f, palette);
                    let seg_bottom = inner.bottom() - i as f32 * seg_h;
                    let seg_rect = Rect::from_min_max(
                        Pos2::new(
                            inner.left() + 1.0,
                            (seg_bottom - seg_h + 1.5).max(inner.top()),
                        ),
                        Pos2::new(inner.right() - 1.0, seg_bottom - 0.5),
                    );
                    painter.rect_filled(seg_rect, CornerRadius::same(1), color);
                }
            }
            if level.held >= 0.001 {
                let y = inner.bottom() - meter_fraction(level.held) * inner.height();
                painter.line_segment(
                    [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
                    Stroke::new(2.0, palette.text_primary),
                );
            }
            painter.rect_stroke(
                bar_rect,
                CornerRadius::same(4),
                Stroke::new(1.0, palette.meter_border),
                egui::StrokeKind::Inside,
            );
        };

        let left_bar = Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(bar_w, bar_h));
        draw_bar(left_bar, levels[0], if stereo { "V" } else { "M" });

        let scale_x = left_bar.right() + 6.0 + scale_w * 0.5;
        for &db in &[0.0_f32, -6.0, -12.0, -18.0, -24.0, -36.0, -48.0, -60.0] {
            let y = top + bar_h * (1.0 - db_to_meter_fraction(db));
            painter.text(
                Pos2::new(scale_x, y),
                egui::Align2::CENTER_CENTER,
                format!("{}", db as i32),
                FontId::proportional(9.5),
                palette.text_secondary,
            );
        }

        if stereo {
            let right_bar = Rect::from_min_size(
                Pos2::new(left_bar.right() + scale_w + 12.0, top),
                Vec2::new(bar_w, bar_h),
            );
            draw_bar(right_bar, levels[1], "H");
        }
    });
}

fn connection_test_row(
    ui: &mut EguiUi,
    dest: &Destination,
    tester: &mut ConnectionTester,
    palette: Palette,
) {
    let status = tester.status(dest.id);
    let testing = matches!(status, Some(ConnectionProbeStatus::Testing));
    let can_test =
        !testing && server_hint(&dest.server).is_none() && mount_hint(&dest.mount).is_none();
    if with_disabled_reason(
        compact_button(ui, "Testa anslutning", can_test, None, palette),
        can_test,
        if testing {
            Some("Test pågår…")
        } else {
            Some("Fyll i en giltig serveradress och monteringspunkt först")
        },
    )
    .clicked()
    {
        tester.start(ui.ctx(), dest);
    }
    match status {
        Some(ConnectionProbeStatus::Testing) => {
            ui.spinner();
            ui.label(
                RichText::new("Testar…")
                    .size(FONT_CAPTION)
                    .color(palette.text_secondary),
            );
        }
        Some(ConnectionProbeStatus::Ok(msg)) => {
            dot_pill(
                ui,
                &msg,
                palette.stream_accent,
                palette.stream_pill_bg,
                palette.stream_active_border,
                palette.stream_accent,
            );
        }
        Some(ConnectionProbeStatus::Failed(err)) => {
            dot_pill(
                ui,
                &err,
                palette.danger,
                palette.danger_pill_bg,
                palette.danger_pill_border,
                palette.danger,
            );
        }
        None => {}
    }
}

fn paint_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: Stroke,
) {
    let steps = 12;
    let points: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let angle = start_angle + (end_angle - start_angle) * t;
            Pos2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect();
    painter.add(Shape::line(points, stroke));
}

#[derive(Clone, Copy)]
enum ButtonVariant {
    Neutral,
    Primary,
    StreamActive,
    RecordActive,
    Danger,
    Saved,
}

fn paint_focus_ring(painter: &egui::Painter, rect: Rect, corner_radius: u8, palette: Palette) {
    painter.rect_stroke(
        rect.expand(1.0),
        CornerRadius::same(corner_radius.saturating_add(1)),
        Stroke::new(2.0, palette.stream_accent),
        egui::StrokeKind::Outside,
    );
}

fn header_count(
    ui: &mut EguiUi,
    active: usize,
    total: usize,
    active_color: Color32,
    palette: Palette,
) -> egui::Response {
    let color = if active > 0 {
        active_color
    } else {
        palette.text_secondary
    };
    ui.label(
        RichText::new(format!("{active}/{total}"))
            .monospace()
            .size(12.0)
            .color(color),
    )
}

fn with_disabled_reason(
    response: egui::Response,
    enabled: bool,
    reason: Option<&str>,
) -> egui::Response {
    if enabled {
        response
    } else if let Some(reason) = reason {
        // Custom disabled buttons allocate with Sense::hover() inside an enabled Ui,
        // so response.enabled() is still true; attach both hover and disabled-hover
        // tooltips so the reason always shows when hovered.
        response
            .on_hover_text(reason)
            .on_disabled_hover_text(reason)
    } else {
        response
    }
}

fn header_pill<R>(
    ui: &mut EguiUi,
    id_salt: &str,
    palette: Palette,
    add_contents: impl FnOnce(&mut EguiUi) -> R,
) -> R {
    let pill_size = Vec2::new(80.0, 34.0);
    let (rect, _) = ui.allocate_exact_size(pill_size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(8), palette.pill_bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, palette.pill_border),
        egui::StrokeKind::Inside,
    );
    let inner_rect = rect.shrink2(Vec2::new(4.0, 3.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id_salt)
            .max_rect(inner_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 4.0;
    add_contents(&mut child)
}

fn icon_button(
    ui: &mut EguiUi,
    icon: Icon,
    enabled: bool,
    icon_color: Color32,
    active_fill: bool,
    palette: Palette,
    tooltip: &str,
) -> egui::Response {
    let is_enabled = enabled && ui.is_enabled();
    let size = Vec2::new(34.0, 28.0);
    let sense = if is_enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, tooltip));

    let (bg, border, fg) = if !is_enabled {
        (
            palette.btn_disabled_bg,
            palette.btn_disabled_border,
            palette.text_disabled,
        )
    } else if active_fill {
        (
            if response.hovered() {
                palette.stream_active_hover
            } else {
                palette.stream_active_bg
            },
            palette.stream_active_border,
            Color32::WHITE,
        )
    } else if response.is_pointer_button_down_on() {
        (palette.btn_bg_active, palette.btn_border_hover, icon_color)
    } else if response.hovered() {
        (palette.btn_bg_hover, palette.btn_border_hover, icon_color)
    } else {
        (palette.btn_bg, palette.btn_border, icon_color)
    };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    if response.has_focus() {
        paint_focus_ring(painter, rect, 6, palette);
    }

    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
    paint_icon(painter, icon_rect, icon, fg);

    response.on_hover_text(tooltip)
}

const BUTTON_ICON_SIZE: f32 = 15.0;
const BUTTON_ICON_GAP: f32 = 7.0;

struct ButtonSpec<'a> {
    icon: Option<(Icon, Color32)>,
    label: &'a str,
    enabled: bool,
    variant: ButtonVariant,
    text_color_override: Option<Color32>,
    height: f32,
    font_size: f32,
    pad_x: f32,
}

fn render_button(ui: &mut EguiUi, spec: ButtonSpec<'_>, palette: Palette) -> egui::Response {
    render_button_sized(ui, spec, None, palette)
}

fn render_button_sized(
    ui: &mut EguiUi,
    spec: ButtonSpec<'_>,
    fixed_width: Option<f32>,
    palette: Palette,
) -> egui::Response {
    let is_saved = matches!(spec.variant, ButtonVariant::Saved);
    let is_enabled = spec.enabled && ui.is_enabled() && !is_saved;
    let font_id = FontId::proportional(spec.font_size);
    let text_galley =
        ui.painter()
            .layout_no_wrap(spec.label.to_owned(), font_id, Color32::PLACEHOLDER);
    let content_width = if spec.icon.is_some() {
        BUTTON_ICON_SIZE + BUTTON_ICON_GAP + text_galley.size().x
    } else {
        text_galley.size().x
    };
    let width = fixed_width.unwrap_or(content_width + spec.pad_x * 2.0);

    let sense = if is_enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, spec.height), sense);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, spec.label)
    });

    let icon_color = spec.icon.map_or(palette.text_primary, |(_, color)| color);
    let (bg, border, text_color, fg_icon) = if is_saved {
        (
            palette.stream_pill_bg,
            palette.stream_active_border,
            palette.stream_accent,
            palette.stream_accent,
        )
    } else if !is_enabled {
        (
            palette.btn_disabled_bg,
            palette.btn_disabled_border,
            palette.text_disabled,
            palette.text_disabled,
        )
    } else {
        match spec.variant {
            ButtonVariant::Neutral => {
                let bg = if response.is_pointer_button_down_on() {
                    palette.btn_bg_active
                } else if response.hovered() {
                    palette.btn_bg_hover
                } else {
                    palette.btn_bg
                };
                let border = if response.hovered() {
                    palette.btn_border_hover
                } else {
                    palette.btn_border
                };
                (
                    bg,
                    border,
                    spec.text_color_override.unwrap_or(palette.text_primary),
                    icon_color,
                )
            }
            ButtonVariant::Primary => {
                let bg = if response.hovered() {
                    palette.stream_active_hover
                } else {
                    palette.tab_active_bg
                };
                (
                    bg,
                    palette.tab_active_border,
                    palette.tab_active_fg,
                    icon_color,
                )
            }
            ButtonVariant::StreamActive => {
                let bg = if response.hovered() {
                    palette.stream_active_hover
                } else {
                    palette.stream_active_bg
                };
                (
                    bg,
                    palette.stream_active_border,
                    Color32::WHITE,
                    Color32::WHITE,
                )
            }
            ButtonVariant::RecordActive | ButtonVariant::Danger => {
                let bg = if response.hovered() {
                    palette.record_active_hover
                } else {
                    palette.record_active_bg
                };
                (
                    bg,
                    palette.record_active_border,
                    Color32::WHITE,
                    Color32::WHITE,
                )
            }
            ButtonVariant::Saved => unreachable!(),
        }
    };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(7), bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(7),
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    if response.has_focus() {
        paint_focus_ring(painter, rect, 7, palette);
    }

    let content_left = rect.center().x - content_width * 0.5;
    if let Some((icon, _)) = spec.icon {
        let icon_center = Pos2::new(content_left + BUTTON_ICON_SIZE * 0.5, rect.center().y);
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(BUTTON_ICON_SIZE));
        paint_icon(painter, icon_rect, icon, fg_icon);

        let text_pos = Pos2::new(
            content_left + BUTTON_ICON_SIZE + BUTTON_ICON_GAP,
            rect.center().y - text_galley.size().y * 0.5,
        );
        painter.galley(text_pos, text_galley, text_color);
    } else {
        let text_pos = Pos2::new(content_left, rect.center().y - text_galley.size().y * 0.5);
        painter.galley(text_pos, text_galley, text_color);
    }

    response
}

fn saved_button(ui: &mut EguiUi, label: &str, palette: Palette) -> egui::Response {
    render_button(
        ui,
        ButtonSpec {
            icon: Some((Icon::Check, palette.stream_accent)),
            label,
            enabled: false,
            variant: ButtonVariant::Saved,
            text_color_override: None,
            height: 30.0,
            font_size: 13.5,
            pad_x: 11.0,
        },
        palette,
    )
}

fn action_button(
    ui: &mut EguiUi,
    icon: impl Into<Option<Icon>>,
    label: &str,
    enabled: bool,
    icon_color: Color32,
    variant: ButtonVariant,
    palette: Palette,
) -> egui::Response {
    render_button(
        ui,
        ButtonSpec {
            icon: icon.into().map(|icon| (icon, icon_color)),
            label,
            enabled,
            variant,
            text_color_override: None,
            height: 30.0,
            font_size: 13.5,
            pad_x: 11.0,
        },
        palette,
    )
}

fn text_button(
    ui: &mut EguiUi,
    label: &str,
    enabled: bool,
    variant: ButtonVariant,
    palette: Palette,
) -> egui::Response {
    render_button(
        ui,
        ButtonSpec {
            icon: None,
            label,
            enabled,
            variant,
            text_color_override: None,
            height: 30.0,
            font_size: 13.5,
            pad_x: 11.0,
        },
        palette,
    )
}

fn neutral_button_colored(
    ui: &mut EguiUi,
    label: &str,
    enabled: bool,
    text_color: Color32,
    palette: Palette,
) -> egui::Response {
    render_button(
        ui,
        ButtonSpec {
            icon: None,
            label,
            enabled,
            variant: ButtonVariant::Neutral,
            text_color_override: Some(text_color),
            height: 30.0,
            font_size: 13.5,
            pad_x: 11.0,
        },
        palette,
    )
}

fn compact_button(
    ui: &mut EguiUi,
    label: &str,
    enabled: bool,
    text_color: Option<Color32>,
    palette: Palette,
) -> egui::Response {
    render_button(
        ui,
        ButtonSpec {
            icon: None,
            label,
            enabled,
            variant: ButtonVariant::Neutral,
            text_color_override: text_color,
            height: 25.0,
            font_size: 12.5,
            pad_x: 9.0,
        },
        palette,
    )
}

fn tab_button(ui: &mut EguiUi, label: &str, selected: bool, palette: Palette) -> egui::Response {
    let font_id = if selected {
        FontId::new(13.5, FontFamily::Name(STRONG_FONT.into()))
    } else {
        FontId::proportional(13.5)
    };
    let text_galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font_id, Color32::PLACEHOLDER);
    let pad_x = 12.0;
    let width = text_galley.size().x + pad_x * 2.0;
    let height = 30.0;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });

    let (bg, border, text_color) = if selected {
        (
            palette.tab_active_bg,
            palette.tab_active_border,
            palette.tab_active_fg,
        )
    } else if response.hovered() {
        (
            palette.btn_bg_hover,
            palette.btn_border,
            palette.text_primary,
        )
    } else {
        (
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
            palette.text_secondary,
        )
    };

    let painter = ui.painter();
    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect, CornerRadius::same(7), bg);
    }
    if border != Color32::TRANSPARENT {
        painter.rect_stroke(
            rect,
            CornerRadius::same(7),
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
    }
    if response.has_focus() {
        paint_focus_ring(painter, rect, 7, palette);
    }

    let text_pos = Pos2::new(
        rect.center().x - text_galley.size().x * 0.5,
        rect.center().y - text_galley.size().y * 0.5,
    );
    painter.galley(text_pos, text_galley, text_color);

    response
}

fn status_pill(
    ui: &mut EguiUi,
    text: impl Into<RichText>,
    bg: Color32,
    border: Color32,
    fg: Color32,
) -> egui::Response {
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(text.into().size(12.0).color(fg));
        })
        .response
}

fn empty_sources_view(ui: &mut EguiUi, palette: Palette, on_add: impl FnOnce()) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.label(strong_text(
            "Lägg till en ljudkälla",
            18.0,
            palette.text_primary,
        ));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Börja med att välja vilken mikrofon du vill använda.")
                .color(palette.text_secondary),
        );
        crate::guide::tip(
            ui,
            "Klicka på Lägg till ljudkälla. Välj sedan din mikrofon i listan.",
        );
        ui.add_space(10.0);
        if action_button(
            ui,
            Icon::Plus,
            "Lägg till ljudkälla",
            true,
            palette.tab_active_fg,
            ButtonVariant::Primary,
            palette,
        )
        .clicked()
        {
            on_add();
        }
    });
}

fn destination_choice_label(dest: &Destination) -> String {
    let name = dest.display_name();
    let mount = dest.mount.trim();
    if mount.is_empty() {
        name.to_owned()
    } else {
        format!("{name} ({mount})")
    }
}

fn destination_summary(dest: &Destination) -> String {
    let public_server = dest.server_without_secrets();
    let server = public_server
        .as_deref()
        .unwrap_or(&dest.server)
        .trim()
        .trim_end_matches('/');
    let mount = dest.mount.trim();
    let endpoint = if mount.is_empty() {
        server.to_owned()
    } else if mount.starts_with('/') {
        format!("{server}{mount}")
    } else {
        format!("{server}/{mount}")
    };
    format!("{} · {} kbit/s · {endpoint}", dest.codec, dest.bitrate)
}

fn next_destination_id(destinations: &[Destination]) -> u64 {
    let mut id = 1;
    while destinations.iter().any(|dest| dest.id == id) {
        id += 1;
    }
    id
}

fn source_stream_disabled_reason(
    source: &Source,
    dest_count: usize,
    streaming: bool,
) -> Option<&'static str> {
    if streaming {
        None
    } else if source.device.is_empty() {
        Some("Välj en ljudenhet i inställningarna")
    } else if dest_count == 0 {
        Some("Lägg till minst en destination")
    } else {
        None
    }
}

fn source_record_disabled_reason(source: &Source, recording: bool) -> Option<&'static str> {
    if recording {
        None
    } else if source.device.is_empty() {
        Some("Välj en ljudenhet i inställningarna")
    } else if source.recording.folder.trim().is_empty() {
        Some("Välj en inspelningsmapp")
    } else {
        None
    }
}

#[derive(Default)]
struct SourceCardActions {
    open_settings: bool,
    set_stream: Option<bool>,
    set_record: Option<bool>,
}

#[allow(clippy::too_many_arguments)]
fn source_card(
    ui: &mut EguiUi,
    engine: &Engine,
    source: &Source,
    destinations: &[Destination],
    streaming: bool,
    recording: bool,
    show_settings_button: bool,
    compact: bool,
    narrow: bool,
    palette: Palette,
) -> SourceCardActions {
    let mut actions = SourceCardActions::default();
    let live = engine.live_status(source.id);
    let resolved = source.resolve_destinations(destinations);
    let count = resolved.len();
    let show_destinations = |ui: &mut EguiUi| {
        let total = if streaming { live.targets.len() } else { count };
        let noun = if narrow {
            "mål"
        } else if total == 1 {
            "destination"
        } else {
            "destinationer"
        };
        let label = if streaming {
            format!("{}/{total} {noun}", live.connected)
        } else {
            format!("{total} {noun}")
        };
        let pill = status_pill(
            ui,
            label,
            palette.pill_bg,
            palette.pill_border,
            palette.text_secondary,
        );
        if total > 0 {
            pill.on_hover_ui(|ui| {
                if streaming {
                    for target in &live.targets {
                        let state = if target.sending {
                            "Ansluten"
                        } else {
                            "Ej ansluten"
                        };
                        ui.label(format!("{} · {state}", target.name));
                    }
                } else {
                    for dest in &resolved {
                        ui.label(dest.display_name());
                    }
                }
            });
        }
    };
    let show_activity = |ui: &mut EguiUi| {
        if streaming {
            let stream_text = if live.connected > 0 {
                format!("Sänder · {}", format_hms(live.stream_elapsed))
            } else {
                "Sändning begärd".to_owned()
            };
            dot_pill(
                ui,
                &stream_text,
                palette.stream_accent,
                palette.stream_pill_bg,
                palette.stream_active_border,
                palette.stream_accent,
            );
        }
        if recording {
            let rec_text = format!("Spelar in · {}", format_hms(live.elapsed));
            dot_pill(
                ui,
                &rec_text,
                palette.record_accent,
                palette.record_pill_bg,
                palette.record_active_border,
                palette.record_accent,
            );
        }
        if streaming && (narrow || compact) {
            show_destinations(ui);
        }
    };
    let border_color = if recording {
        palette.record_active_border
    } else if streaming {
        palette.stream_active_border
    } else {
        palette.card_border
    };

    let margin = if compact || narrow {
        Margin::symmetric(10, 8)
    } else {
        Margin::symmetric(14, 10)
    };
    let frame = card_frame(palette, border_color).inner_margin(margin);
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = if compact { 4.0 } else { 6.0 };

        let present = live.present;
        let ready = present && !streaming && !recording;
        let show_ready = |ui: &mut EguiUi| {
            if ready {
                status_pill(
                    ui,
                    "Redo",
                    palette.pill_bg,
                    palette.pill_border,
                    palette.text_secondary,
                );
            }
        };
        let mut levels = engine.levels(source.id);
        if !present {
            for level in &mut levels {
                level.signal = 0.0;
            }
        }
        let stereo = source.right.is_some();

        if narrow && !compact {
            // Narrow docked card: single-line header + slim labeled meter + equal-width split toggles
            ui.horizontal(|ui| {
                let controls_w = if show_settings_button { 118.0 } else { 80.0 };
                let title_max_w = (ui.available_width() - controls_w).max(60.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(title_max_w, 28.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(strong_text(
                                source.display_name(),
                                FONT_TITLE_S,
                                palette.text_primary,
                            ))
                            .truncate(),
                        );
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if show_settings_button
                        && icon_button(
                            ui,
                            Icon::Settings,
                            true,
                            palette.text_secondary,
                            false,
                            palette,
                            "Inställningar",
                        )
                        .clicked()
                    {
                        actions.open_settings = true;
                    }
                    show_ready(ui);
                });
            });
            if streaming || recording {
                ui.horizontal_wrapped(|ui| show_activity(ui));
            }

            source_status_notices(ui, engine, source, streaming, recording, &live, palette);

            // Narrow stereo/mono meter strip with tiny V/H tag + dB readout
            ui.push_id(source.id, |ui| {
                let channels: &[MeterLevel] = &levels[..if stereo { 2 } else { 1 }];
                for (ch, &level) in channels.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let tag = match (stereo, ch) {
                            (true, 0) => "V",
                            (true, _) => "H",
                            _ => "M",
                        };
                        ui.add_sized(
                            [14.0, 12.0],
                            egui::Label::new(
                                RichText::new(tag)
                                    .size(11.0)
                                    .color(palette.text_secondary),
                            ),
                        );
                        let bar_w = (ui.available_width() - 68.0).max(40.0);
                        let (bar_rect, _) =
                            ui.allocate_exact_size(Vec2::new(bar_w, 8.0), Sense::hover());
                        paint_meter_bar(ui, bar_rect, level, false, palette);
                        let short_db = if level.signal.is_finite() && level.signal >= 0.001 {
                            format!("{:.0} dB", 20.0 * level.signal.log10())
                        } else {
                            "−∞ dB".to_owned()
                        };
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(short_db)
                                    .monospace()
                                    .size(11.0)
                                    .color(palette.text_secondary),
                            );
                        });
                    });
                }
                for (channel, level) in channels.iter().enumerate() {
                    if level.clipped {
                        let label = match (stereo, channel) {
                            (true, 0) => "Klipp V · Återställ",
                            (true, _) => "Klipp H · Återställ",
                            _ => "Klipp · Återställ",
                        };
                        if compact_button(ui, label, true, Some(palette.danger), palette).clicked()
                        {
                            engine.reset_clip(source.id, channel);
                        }
                    }
                }
            });

            // Side-by-side equal-width split controls
            let stream_enabled = streaming || (!source.device.is_empty() && count > 0);
            let record_enabled = recording
                || (!source.device.is_empty() && !source.recording.folder.trim().is_empty());
            let stream_reason = source_stream_disabled_reason(source, count, streaming);
            let record_reason = source_record_disabled_reason(source, recording);
            let gap = 8.0;
            let btn_w = ((ui.available_width() - gap) * 0.5).floor().max(80.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let (s_icon, s_label, s_var) = if streaming {
                    (Icon::Stop, "Sändning", ButtonVariant::StreamActive)
                } else {
                    (Icon::Broadcast, "Sändning", ButtonVariant::Neutral)
                };
                if with_disabled_reason(
                    render_button_sized(
                        ui,
                        ButtonSpec {
                            icon: Some((s_icon, palette.stream_accent)),
                            label: s_label,
                            enabled: stream_enabled,
                            variant: s_var,
                            text_color_override: None,
                            height: 30.0,
                            font_size: FONT_BODY,
                            pad_x: 8.0,
                        },
                        Some(btn_w),
                        palette,
                    ),
                    stream_enabled,
                    stream_reason,
                )
                .clicked()
                {
                    actions.set_stream = Some(!streaming);
                }

                let (r_icon, r_label, r_var) = if recording {
                    (Icon::Stop, "Inspelning", ButtonVariant::RecordActive)
                } else {
                    (Icon::Record, "Spela in", ButtonVariant::Neutral)
                };
                if with_disabled_reason(
                    render_button_sized(
                        ui,
                        ButtonSpec {
                            icon: Some((r_icon, palette.record_accent)),
                            label: r_label,
                            enabled: record_enabled,
                            variant: r_var,
                            text_color_override: None,
                            height: 30.0,
                            font_size: FONT_BODY,
                            pad_x: 8.0,
                        },
                        Some(btn_w),
                        palette,
                    ),
                    record_enabled,
                    record_reason,
                )
                .clicked()
                {
                    actions.set_record = Some(!recording);
                }
            });
            return;
        }

        // Standard / Compact / Wide card header
        if compact && narrow {
            ui.horizontal(|ui| {
                let controls_w = if show_settings_button { 104.0 } else { 70.0 };
                let ready_w = if ready { 54.0 } else { 0.0 };
                let title_max_w = (ui.available_width() - controls_w - ready_w).max(60.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(title_max_w, 28.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(strong_text(
                                source.display_name(),
                                FONT_TITLE_S,
                                palette.text_primary,
                            ))
                            .truncate(),
                        );
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if show_settings_button
                        && icon_button(
                            ui,
                            Icon::Settings,
                            true,
                            palette.text_secondary,
                            false,
                            palette,
                            "Inställningar",
                        )
                        .clicked()
                    {
                        actions.open_settings = true;
                    }
                    source_output_controls(
                        ui,
                        source,
                        count,
                        streaming,
                        recording,
                        true,
                        palette,
                        &mut actions,
                    );
                    show_ready(ui);
                });
            });
            if streaming || recording {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    show_activity(ui);
                });
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(strong_text(
                    source.display_name(),
                    FONT_TITLE_M,
                    palette.text_primary,
                ));
                crate::guide::tip(ui, "Starta inspelning sparar ljudet på datorn. Starta sändning låter andra lyssna direkt.");
                show_ready(ui);
                show_activity(ui);

                if compact {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if show_settings_button
                            && icon_button(
                                ui,
                                Icon::Settings,
                                true,
                                palette.text_secondary,
                                false,
                                palette,
                                "Inställningar",
                            )
                            .clicked()
                        {
                            actions.open_settings = true;
                        }
                        source_output_controls(
                            ui,
                            source,
                            count,
                            streaming,
                            recording,
                            true,
                            palette,
                            &mut actions,
                        );
                    });
                } else if show_settings_button {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if action_button(
                            ui,
                            Icon::Settings,
                            "Inställningar",
                            true,
                            palette.text_secondary,
                            ButtonVariant::Neutral,
                            palette,
                        )
                        .clicked()
                        {
                            actions.open_settings = true;
                        }
                    });
                }
            });
        }

        if !compact {
            let (sep_rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, palette.card_divider);
        }

        source_status_notices(ui, engine, source, streaming, recording, &live, palette);
        ui.push_id(source.id, |ui| {
            if compact {
                compact_meter(ui, &levels[..if stereo { 2 } else { 1 }], palette);
            } else {
                ui.spacing_mut().item_spacing.y = 4.0;
                meter(
                    ui,
                    if stereo { "Vänster" } else { "Mono" },
                    levels[0],
                    !stereo,
                    palette,
                );
                if stereo {
                    meter(ui, "Höger", levels[1], true, palette);
                }
            }
            for (channel, level) in levels.iter().take(if stereo { 2 } else { 1 }).enumerate() {
                if level.clipped {
                    let label = match (stereo, channel) {
                        (true, 0) => "Klippning vänster · Återställ",
                        (true, _) => "Klippning höger · Återställ",
                        _ => "Klippning · Återställ",
                    };
                    if compact_button(ui, label, true, Some(palette.danger), palette).clicked() {
                        engine.reset_clip(source.id, channel);
                    }
                }
            }
        });

        if !compact {
            ui.horizontal_wrapped(|ui| {
                source_output_controls(
                    ui,
                    source,
                    count,
                    streaming,
                    recording,
                    false,
                    palette,
                    &mut actions,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), show_destinations);
            });
        }
    });
    actions
}

#[allow(clippy::too_many_arguments)]
fn source_output_controls(
    ui: &mut EguiUi,
    source: &Source,
    count: usize,
    streaming: bool,
    recording: bool,
    compact: bool,
    palette: Palette,
    actions: &mut SourceCardActions,
) {
    let stream_enabled = streaming || (!source.device.is_empty() && count > 0);
    let stream_reason = source_stream_disabled_reason(source, count, streaming);
    let (stream_icon, stream_label, stream_variant) = if streaming {
        (Icon::Stop, "Stoppa sändning", ButtonVariant::StreamActive)
    } else {
        (Icon::Broadcast, "Starta sändning", ButtonVariant::Neutral)
    };
    if with_disabled_reason(
        if compact {
            icon_button(
                ui,
                if streaming {
                    Icon::BroadcastStop
                } else {
                    Icon::Broadcast
                },
                stream_enabled,
                palette.stream_accent,
                streaming,
                palette,
                stream_label,
            )
        } else {
            action_button(
                ui,
                stream_icon,
                stream_label,
                stream_enabled,
                palette.stream_accent,
                stream_variant,
                palette,
            )
            .on_hover_text("De som lyssnar kan höra ljudet medan du sänder.")
        },
        stream_enabled,
        stream_reason,
    )
    .clicked()
    {
        actions.set_stream = Some(!streaming);
    }

    let record_enabled =
        recording || (!source.device.is_empty() && !source.recording.folder.trim().is_empty());
    let record_reason = source_record_disabled_reason(source, recording);
    let (record_icon, record_label, record_variant) = if recording {
        (Icon::Stop, "Stoppa inspelning", ButtonVariant::RecordActive)
    } else {
        (Icon::Record, "Starta inspelning", ButtonVariant::Neutral)
    };
    if with_disabled_reason(
        if compact {
            icon_button(
                ui,
                if recording {
                    Icon::RecordStop
                } else {
                    Icon::Record
                },
                record_enabled,
                palette.record_accent,
                false,
                palette,
                record_label,
            )
        } else {
            action_button(
                ui,
                record_icon,
                record_label,
                record_enabled,
                palette.record_accent,
                record_variant,
                palette,
            )
            .on_hover_text("Ljudet sparas i en fil på din dator.")
        },
        record_enabled,
        record_reason,
    )
    .clicked()
    {
        actions.set_record = Some(!recording);
    }
}

const METER_MIN_DB: f32 = -60.0;
const METER_MAX_DB: f32 = 0.0;
const METER_TICKS_DB: &[f32] = &[
    -60.0, -54.0, -48.0, -42.0, -36.0, -30.0, -24.0, -18.0, -12.0, -6.0, -3.0, 0.0,
];
// Numeric scale labels in placement priority order so endpoints and key dB landmarks
// stay visible while colliding labels drop cleanly on narrow widths.
const METER_SCALE_LABELS: &[(f32, &str)] = &[
    (0.0, "0"),
    (-60.0, "-60"),
    (-6.0, "-6"),
    (-12.0, "-12"),
    (-24.0, "-24"),
    (-36.0, "-36"),
    (-48.0, "-48"),
    (-18.0, "-18"),
    (-3.0, "-3"),
];

fn db_to_meter_fraction(db: f32) -> f32 {
    if db.is_finite() {
        ((db - METER_MIN_DB) / (METER_MAX_DB - METER_MIN_DB)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn meter_fraction(signal: f32) -> f32 {
    if signal.is_finite() && signal > 0.0 {
        db_to_meter_fraction(20.0 * signal.log10())
    } else {
        0.0
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |c0: u8, c1: u8| -> u8 {
        (f32::from(c0) + (f32::from(c1) - f32::from(c0)) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_premultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

fn meter_gradient_stops(palette: Palette) -> [(f32, Color32); 6] {
    [
        (db_to_meter_fraction(-60.0), palette.meter_ok),
        (db_to_meter_fraction(-24.0), palette.meter_ok),
        (db_to_meter_fraction(-18.0), palette.meter_warn),
        (db_to_meter_fraction(-6.0), palette.meter_warn),
        (db_to_meter_fraction(-3.0), palette.meter_peak),
        (db_to_meter_fraction(0.0), palette.meter_peak),
    ]
}

fn meter_gradient_color(frac: f32, palette: Palette) -> Color32 {
    let frac = frac.clamp(0.0, 1.0);
    let stops = meter_gradient_stops(palette);
    for pair in stops.windows(2) {
        let (f0, c0) = pair[0];
        let (f1, c1) = pair[1];
        if frac <= f1 {
            let span = f1 - f0;
            let t = if span > 0.0 { (frac - f0) / span } else { 0.0 };
            return lerp_color(c0, c1, t);
        }
    }
    stops[stops.len() - 1].1
}

fn build_meter_gradient_mesh(rect: Rect, fill_frac: f32, palette: Palette) -> egui::epaint::Mesh {
    let mut mesh = egui::epaint::Mesh::default();
    let fill_frac = fill_frac.clamp(0.0, 1.0);
    if fill_frac <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return mesh;
    }

    let stops = meter_gradient_stops(palette);
    for pair in stops.windows(2) {
        let (f0, c0) = pair[0];
        let (f1, _) = pair[1];
        if f0 >= fill_frac {
            break;
        }
        let end_frac = f1.min(fill_frac);
        if end_frac <= f0 {
            continue;
        }
        let c1 = meter_gradient_color(end_frac, palette);
        let x0 = rect.left() + rect.width() * f0;
        let x1 = rect.left() + rect.width() * end_frac;
        let base = mesh.vertices.len() as u32;
        mesh.vertices.extend_from_slice(&[
            egui::epaint::Vertex {
                pos: Pos2::new(x0, rect.top()),
                uv: egui::epaint::WHITE_UV,
                color: c0,
            },
            egui::epaint::Vertex {
                pos: Pos2::new(x1, rect.top()),
                uv: egui::epaint::WHITE_UV,
                color: c1,
            },
            egui::epaint::Vertex {
                pos: Pos2::new(x1, rect.bottom()),
                uv: egui::epaint::WHITE_UV,
                color: c1,
            },
            egui::epaint::Vertex {
                pos: Pos2::new(x0, rect.bottom()),
                uv: egui::epaint::WHITE_UV,
                color: c0,
            },
        ]);
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    mesh
}

struct PlacedScaleLabel {
    index: usize,
    tick_x: f32,
    label_x: f32,
    width: f32,
}

fn layout_meter_scale_labels(
    bar_left: f32,
    bar_width: f32,
    candidates: &[(f32, f32)],
    min_gap: f32,
) -> Vec<PlacedScaleLabel> {
    if bar_width <= 0.0 {
        return Vec::new();
    }
    let bar_right = bar_left + bar_width;
    let mut placed: Vec<PlacedScaleLabel> = Vec::with_capacity(candidates.len());

    for (index, &(db, width)) in candidates.iter().enumerate() {
        let frac = db_to_meter_fraction(db);
        let tick_x = bar_left + bar_width * frac;
        let max_left = (bar_right - width).max(bar_left);
        let label_x = if frac <= 0.001 {
            bar_left
        } else if frac >= 0.999 {
            max_left
        } else {
            (tick_x - width * 0.5).clamp(bar_left, max_left)
        };
        let label_right = label_x + width;
        let overlaps = placed.iter().any(|other| {
            label_x < other.label_x + other.width + min_gap && other.label_x < label_right + min_gap
        });
        if !overlaps {
            placed.push(PlacedScaleLabel {
                index,
                tick_x,
                label_x,
                width,
            });
        }
    }

    placed.sort_by(|a, b| a.label_x.total_cmp(&b.label_x));
    placed
}

fn paint_meter_scale(painter: &egui::Painter, bar_rect: Rect, scale_top: f32, palette: Palette) {
    let font_id = FontId::proportional(9.5);
    let galleys: Vec<Arc<egui::Galley>> = METER_SCALE_LABELS
        .iter()
        .map(|&(_, text)| {
            painter.layout_no_wrap(text.to_owned(), font_id.clone(), Color32::PLACEHOLDER)
        })
        .collect();
    let candidates: Vec<(f32, f32)> = METER_SCALE_LABELS
        .iter()
        .zip(&galleys)
        .map(|(&(db, _), galley)| (db, galley.size().x))
        .collect();

    let placed = layout_meter_scale_labels(bar_rect.left(), bar_rect.width(), &candidates, 4.0);
    for item in placed {
        let tick_x = item
            .tick_x
            .clamp(bar_rect.left() + 0.5, bar_rect.right() - 0.5);
        painter.line_segment(
            [
                Pos2::new(tick_x, bar_rect.bottom()),
                Pos2::new(tick_x, bar_rect.bottom() + 2.5),
            ],
            Stroke::new(1.0, palette.meter_border),
        );
        painter.galley(
            Pos2::new(item.label_x, scale_top),
            Arc::clone(&galleys[item.index]),
            palette.text_secondary,
        );
    }
}

fn paint_meter_bar(
    ui: &EguiUi,
    bar_rect: Rect,
    level: MeterLevel,
    show_ticks: bool,
    palette: Palette,
) {
    let painter = ui.painter();
    let rounding = CornerRadius::same(3);
    painter.rect_filled(bar_rect, rounding, palette.meter_track);

    let fill_rect = bar_rect.shrink(1.0);
    let frac = meter_fraction(level.signal);
    if frac > 0.002 && fill_rect.width() > 0.0 {
        let effective_frac = frac.max(2.0 / fill_rect.width()).min(1.0);
        let mesh = build_meter_gradient_mesh(fill_rect, effective_frac, palette);
        painter.add(Shape::mesh(mesh));
    }

    if show_ticks {
        for &tick_db in METER_TICKS_DB {
            if tick_db <= METER_MIN_DB || tick_db >= METER_MAX_DB {
                continue;
            }
            let tick_frac = db_to_meter_fraction(tick_db);
            let x = fill_rect.left() + fill_rect.width() * tick_frac;
            let major = METER_SCALE_LABELS
                .iter()
                .any(|&(label_db, _)| (label_db - tick_db).abs() < 0.01);
            let covered = frac > 0.002 && tick_frac <= frac;
            let tick_color = match (major, covered) {
                (true, true) => palette.meter_track.gamma_multiply(0.6),
                (true, false) => palette.text_disabled.gamma_multiply(0.5),
                (false, true) => palette.meter_track.gamma_multiply(0.4),
                (false, false) => palette.text_disabled.gamma_multiply(0.32),
            };
            let top_y = if major {
                fill_rect.top() + 0.5
            } else {
                fill_rect.center().y
            };
            painter.line_segment(
                [Pos2::new(x, top_y), Pos2::new(x, fill_rect.bottom() - 0.5)],
                Stroke::new(1.0, tick_color),
            );
        }
    }
    painter.rect_stroke(
        bar_rect,
        rounding,
        Stroke::new(1.0, palette.meter_border),
        egui::StrokeKind::Inside,
    );

    if level.held >= 0.001 {
        let x = fill_rect.left() + meter_fraction(level.held) * fill_rect.width();
        painter.line_segment(
            [
                Pos2::new(x, fill_rect.top()),
                Pos2::new(x, fill_rect.bottom()),
            ],
            Stroke::new(2.0, palette.text_primary),
        );
    }
}

fn compact_meter(ui: &mut EguiUi, levels: &[MeterLevel], palette: Palette) {
    // Stereo lanes share one strip, with left above right.
    let height = 6.0;
    let total_height = if levels.len() == 2 {
        2.0 * height + 1.0
    } else {
        height
    };
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), total_height),
        Sense::hover(),
    );
    for (index, &level) in levels.iter().enumerate() {
        let lane = Rect::from_min_size(
            rect.min + Vec2::new(0.0, index as f32 * (height + 1.0)),
            Vec2::new(rect.width(), height),
        );
        paint_meter_bar(ui, lane, level, false, palette);
    }
    response.on_hover_text(if levels.len() == 2 {
        "Vänster kanal överst, höger kanal nederst. Strecken visar toppnivån."
    } else {
        "Mono. Strecket visar toppnivån."
    });
}

fn meter(ui: &mut EguiUi, label: &str, level: MeterLevel, show_scale: bool, palette: Palette) {
    let signal = level.signal;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.spacing_mut().interact_size.y = 18.0;
        let mut bar_rect = Rect::NOTHING;
        ui.horizontal(|ui| {
            ui.add_sized(
                [54.0, 18.0],
                egui::Label::new(
                    RichText::new(label)
                        .size(12.5)
                        .color(palette.text_secondary),
                ),
            );

            let bar_width = (ui.available_width() - 82.0).max(40.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, 12.0), Sense::hover());
            bar_rect = rect;
            paint_meter_bar(ui, rect, level, true, palette);

            let text = if signal.is_finite() && signal >= 0.001 {
                format!("{:.1} dBFS", 20.0 * signal.log10())
            } else {
                "−∞ dBFS".to_owned()
            };
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(text)
                        .monospace()
                        .size(12.0)
                        .color(palette.text_secondary),
                );
            });
        });

        if show_scale {
            let (scale_rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 13.0), Sense::hover());
            paint_meter_scale(ui.painter(), bar_rect, scale_rect.top(), palette);
        }
    });
}

#[derive(Default)]
struct SourceSettingsOutcome {
    changed: bool,
    refresh_devices: bool,
}

fn device_choice_label(name: &str, channels: usize) -> String {
    if channels == 1 {
        format!("{name} · 1 kanal")
    } else if channels > 1 {
        format!("{name} · {channels} kanaler")
    } else {
        name.to_owned()
    }
}

#[allow(clippy::too_many_arguments)]
fn source_settings(
    ui: &mut EguiUi,
    source: &mut Source,
    destinations: &mut Vec<Destination>,
    new_destination_draft: &mut Option<Destination>,
    inline_edit_destination: &mut Option<u64>,
    connection_tester: &mut ConnectionTester,
    devices: &[(String, String, usize)],
    streaming: bool,
    recording: bool,
    streaming_dest_ids: &HashSet<u64>,
    narrow: bool,
    palette: Palette,
) -> SourceSettingsOutcome {
    let mut outcome = SourceSettingsOutcome::default();
    let input_locked = streaming || recording;
    let narrow = narrow || ui.available_width() < 520.0;

    // Section 1: Ljudingång (Namn stays editable even while live!)
    card_frame(palette, palette.card_border).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(strong_text(
                "Ljudingång",
                FONT_TITLE_S,
                palette.text_primary,
            ));
            crate::guide::tip(
                ui,
                "Välj din mikrofon och prata en stund. Stapeln rör sig när ljudet kommer in.",
            );
            if input_locked {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let lock_text = match (streaming, recording) {
                        (true, true) => "Låst under sändning och inspelning",
                        (true, false) => "Låst under sändning",
                        (false, true) => "Låst under inspelning",
                        (false, false) => unreachable!(),
                    };
                    lock_badge(ui, lock_text, palette);
                });
            }
        });
        ui.add_space(SPACE_S);

        let render_name_row =
            |ui: &mut EguiUi, source: &mut Source, outcome: &mut SourceSettingsOutcome| {
                outcome.changed |= stacked_field(ui, "Namn", &mut source.name, palette).changed();
            };

        let render_device_picker =
            |ui: &mut EguiUi, source: &mut Source, outcome: &mut SourceSettingsOutcome| {
                let mut device = source.device.clone();
                let selected_text = devices
                    .iter()
                    .find(|(id, _, _)| id == &device)
                    .map(|(_, name, ch)| device_choice_label(name, *ch))
                    .unwrap_or_else(|| {
                        if source.device_name.is_empty() {
                            "Välj ljudenhet".to_owned()
                        } else {
                            source.device_name.clone()
                        }
                    });
                let combo_w = (ui.available_width() - 44.0).max(100.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.scope(|ui| {
                        ui.set_width(combo_w);
                        ComboBox::from_id_salt("device")
                            .width((combo_w - 24.0).max(80.0))
                            .truncate()
                            .selected_text(&selected_text)
                            .show_ui(ui, |ui| {
                                if devices.is_empty() {
                                    ui.label(
                                        RichText::new("Inga ljudenheter tillgängliga")
                                            .color(palette.text_secondary),
                                    );
                                }
                                for (id, name, ch) in devices {
                                    let label = device_choice_label(name, *ch);
                                    ui.selectable_value(&mut device, id.clone(), &label)
                                        .on_hover_text(label);
                                }
                            })
                            .response
                            .on_hover_text(&selected_text);
                    });
                    if icon_button(
                        ui,
                        Icon::Refresh,
                        true,
                        palette.text_secondary,
                        false,
                        palette,
                        "Uppdatera ljudenheter",
                    )
                    .clicked()
                    {
                        outcome.refresh_devices = true;
                    }
                });
                if device != source.device
                    && let Some((_, name, count)) = devices.iter().find(|(id, _, _)| id == &device)
                {
                    source.device = device;
                    source.device_name.clone_from(name);
                    source.left = source.left.min(count.saturating_sub(1));
                    source.right = source
                        .right
                        .filter(|right| *right < *count && *right != source.left);
                    outcome.changed = true;
                }
            };

        let render_channels = |ui: &mut EguiUi,
                               source: &mut Source,
                               outcome: &mut SourceSettingsOutcome| {
            let count = devices
                .iter()
                .find(|(id, _, _)| id == &source.device)
                .map_or(0, |(_, _, count)| *count);
            let mut is_stereo = source.right.is_some();
            let stereo_available = count >= 2 || is_stereo;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if segmented_bool(
                    ui,
                    "mono-stereo",
                    &mut is_stereo,
                    "Mono",
                    "Stereo",
                    stereo_available,
                    palette,
                ) {
                    if is_stereo {
                        let candidate = (0..count).find(|&c| c != source.left).unwrap_or(1);
                        source.right = Some(candidate);
                    } else {
                        source.right = None;
                    }
                    outcome.changed = true;
                }

                let left = source.left;
                let old_right = source.right;
                ui.label(
                    RichText::new(if source.right.is_some() {
                        "Vänster:"
                    } else {
                        "Kanal:"
                    })
                    .color(palette.text_secondary),
                );
                ComboBox::from_id_salt("left")
                    .width(56.0)
                    .selected_text(left.saturating_add(1).to_string())
                    .show_ui(ui, |ui| {
                        for channel in 0..count {
                            ui.selectable_value(
                                &mut source.left,
                                channel,
                                (channel + 1).to_string(),
                            );
                        }
                    });
                if left != source.left {
                    if old_right == Some(source.left) {
                        source.right = Some(left);
                    }
                    outcome.changed = true;
                }

                if let Some(right_val) = source.right {
                    ui.label(RichText::new("Höger:").color(palette.text_secondary));
                    let mut chosen_right = right_val;
                    ComboBox::from_id_salt("right")
                        .width(56.0)
                        .selected_text(right_val.saturating_add(1).to_string())
                        .show_ui(ui, |ui| {
                            for channel in (0..count).filter(|channel| *channel != source.left) {
                                ui.selectable_value(
                                    &mut chosen_right,
                                    channel,
                                    (channel + 1).to_string(),
                                );
                            }
                        });
                    if chosen_right != right_val {
                        source.right = Some(chosen_right);
                        outcome.changed = true;
                    }
                }
            });
        };

        if narrow {
            ui.spacing_mut().item_spacing.y = 6.0;
            render_name_row(ui, source, &mut outcome);
            ui.add_enabled_ui(!input_locked, |ui| {
                field_caption(ui, "Ljudenhet", palette);
                render_device_picker(ui, source, &mut outcome);
                field_caption(ui, "Kanaler", palette);
                render_channels(ui, source, &mut outcome);
            });
        } else {
            ui.spacing_mut().item_spacing.y = 8.0;
            let (col_w, gap, spacing_x) = two_col_metrics(ui);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                col_box(ui, col_w, spacing_x, |ui| {
                    render_name_row(ui, source, &mut outcome);
                });
                col_box(ui, col_w, spacing_x, |ui| {
                    ui.add_enabled_ui(!input_locked, |ui| {
                        field_caption(ui, "Ljudenhet", palette);
                        render_device_picker(ui, source, &mut outcome);
                    });
                });
            });
            ui.add_enabled_ui(!input_locked, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    field_caption(ui, "Kanaler", palette);
                    render_channels(ui, source, &mut outcome);
                });
            });
        }
    });

    ui.add_space(10.0);

    // Section 2: Sändningsdestinationer (with INLINE destination editor!)
    card_frame(palette, palette.card_border).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(strong_text(
                "Sändningsdestinationer",
                FONT_TITLE_S,
                palette.text_primary,
            ));
            crate::guide::tip(
                ui,
                "Välj vart ljudet ska sändas. Du kan redigera server och lösenord direkt här.",
            );
            if streaming {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    lock_badge(ui, "Låst under sändning", palette);
                });
            }
        });
        ui.add_space(SPACE_S);
        if source.destination_ids.is_empty() {
            ui.label(
                RichText::new("Ingen destination vald för den här ljudkällan.")
                    .color(palette.text_secondary),
            );
        }
        let mut remove_slot = None;
        let mut replace_slot: Option<(usize, u64)> = None;
        let slot_ids = source.destination_ids.clone();
        for (slot_index, dest_id) in slot_ids.into_iter().enumerate() {
            let Some(dest_pos) = destinations.iter().position(|dest| dest.id == dest_id) else {
                continue;
            };
            let is_editing = *inline_edit_destination == Some(dest_id);
            let dest_streaming = streaming_dest_ids.contains(&dest_id);
            let slot_border = if is_editing {
                palette.stream_active_border
            } else {
                palette.pill_border
            };

            ui.push_id(("dest-slot", slot_index, dest_id), |ui| {
                let frame_resp = Frame::new()
                    .fill(palette.pill_bg)
                    .stroke(Stroke::new(1.0, slot_border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let picker_w = if narrow {
                            (ui.available_width() - 20.0).max(110.0)
                        } else {
                            (ui.available_width() - 220.0).clamp(140.0, 250.0)
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.add_enabled_ui(!streaming, |ui| {
                                let mut chosen = dest_id;
                                let current_label =
                                    destination_choice_label(&destinations[dest_pos]);
                                ui.scope(|ui| {
                                    ui.set_width(picker_w);
                                    ComboBox::from_id_salt("dest-picker")
                                        .width((picker_w - 24.0).max(80.0))
                                        .truncate()
                                        .selected_text(current_label)
                                        .show_ui(ui, |ui| {
                                            for candidate in destinations.iter() {
                                                let used_elsewhere =
                                                    source.destination_ids.iter().enumerate().any(
                                                        |(i, id)| {
                                                            i != slot_index && *id == candidate.id
                                                        },
                                                    );
                                                if !used_elsewhere {
                                                    ui.selectable_value(
                                                        &mut chosen,
                                                        candidate.id,
                                                        destination_choice_label(candidate),
                                                    );
                                                }
                                            }
                                        });
                                });
                                if chosen != dest_id {
                                    replace_slot = Some((slot_index, chosen));
                                }
                            });

                            if compact_button(
                                ui,
                                if is_editing {
                                    "Stäng redigering"
                                } else {
                                    "Redigera"
                                },
                                true,
                                if is_editing {
                                    Some(palette.stream_accent)
                                } else {
                                    None
                                },
                                palette,
                            )
                            .clicked()
                            {
                                *inline_edit_destination =
                                    if is_editing { None } else { Some(dest_id) };
                            }
                            if !streaming
                                && compact_button(
                                    ui,
                                    "Ta bort",
                                    true,
                                    Some(palette.danger),
                                    palette,
                                )
                                .clicked()
                            {
                                remove_slot = Some(slot_index);
                            }
                        });

                        if !is_editing {
                            ui.label(
                                RichText::new(destination_summary(&destinations[dest_pos]))
                                    .monospace()
                                    .size(FONT_CAPTION)
                                    .color(palette.text_secondary),
                            );
                        } else {
                            ui.add_space(SPACE_M);
                            if dest_streaming {
                                lock_badge(
                                    ui,
                                    "Destinationen används av en aktiv sändning",
                                    palette,
                                );
                                ui.add_space(SPACE_S);
                            }
                            let dest = &mut destinations[dest_pos];
                            ui.add_enabled_ui(!dest_streaming, |ui| {
                                ui.push_id(("inline-dest-edit", dest.id), |ui| {
                                    if destination_settings(ui, dest, narrow, palette) {
                                        connection_tester.clear(dest.id);
                                        outcome.changed = true;
                                    }
                                });
                            });
                            ui.add_space(SPACE_S);
                            ui.horizontal_wrapped(|ui| {
                                connection_test_row(ui, dest, connection_tester, palette);
                            });
                        }
                    });

                // Subtle teal left accent stripe while inline editing
                if is_editing {
                    let r = frame_resp.response.rect;
                    let stripe = Rect::from_min_max(
                        Pos2::new(r.left() + 1.0, r.top() + 5.0),
                        Pos2::new(r.left() + 4.0, r.bottom() - 5.0),
                    );
                    ui.painter()
                        .rect_filled(stripe, CornerRadius::same(2), palette.stream_accent);
                }
            });
            ui.add_space(6.0);
        }

        if let Some((slot_index, new_id)) = replace_slot {
            if *inline_edit_destination == Some(source.destination_ids[slot_index]) {
                *inline_edit_destination = Some(new_id);
            }
            source.destination_ids[slot_index] = new_id;
            outcome.changed = true;
        }
        if let Some(slot_index) = remove_slot {
            let removed_id = source.destination_ids.remove(slot_index);
            if *inline_edit_destination == Some(removed_id) {
                *inline_edit_destination = None;
            }
            outcome.changed = true;
        }

        let available: Vec<(u64, String)> = destinations
            .iter()
            .filter(|dest| !source.destination_ids.contains(&dest.id))
            .map(|dest| (dest.id, destination_choice_label(dest)))
            .collect();
        ui.add_enabled_ui(!streaming, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut add_existing = None;
                let add_combo_w = if narrow {
                    (ui.available_width() - 8.0).clamp(140.0, 260.0)
                } else {
                    230.0
                };
                ui.add_enabled_ui(!available.is_empty(), |ui| {
                    ComboBox::from_id_salt("add-existing-dest")
                        .width(add_combo_w)
                        .truncate()
                        .selected_text(if available.is_empty() {
                            if destinations.is_empty() {
                                "Inga sparade destinationer"
                            } else {
                                "Alla destinationer valda"
                            }
                        } else {
                            "Välj befintlig destination…"
                        })
                        .show_ui(ui, |ui| {
                            for (id, label) in &available {
                                if ui.selectable_label(false, label).clicked() {
                                    add_existing = Some(*id);
                                }
                            }
                        });
                });
                if let Some(id) = add_existing {
                    source.destination_ids.push(id);
                    outcome.changed = true;
                }
                if new_destination_draft.is_none()
                    && action_button(
                        ui,
                        Icon::Plus,
                        "Skapa ny destination",
                        true,
                        palette.text_primary,
                        ButtonVariant::Neutral,
                        palette,
                    )
                    .clicked()
                {
                    let next_id = next_destination_id(destinations);
                    *new_destination_draft = Some(Destination {
                        name: format!("Destination {next_id}"),
                        ..Destination::default()
                    });
                }
            });

            if let Some(draft) = new_destination_draft.as_mut() {
                let mut create = false;
                let mut cancel = false;
                ui.add_space(6.0);
                Frame::new()
                    .fill(palette.pill_bg)
                    .stroke(Stroke::new(1.0, palette.stream_active_border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(strong_text(
                            "Ny global destination",
                            14.0,
                            palette.text_primary,
                        ));
                        ui.add_space(4.0);
                        ui.push_id("new-dest-draft", |ui| {
                            destination_settings(ui, draft, narrow, palette);
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if text_button(
                                ui,
                                "Skapa och välj",
                                true,
                                ButtonVariant::Primary,
                                palette,
                            )
                            .clicked()
                            {
                                create = true;
                            }
                            if text_button(ui, "Avbryt", true, ButtonVariant::Neutral, palette)
                                .clicked()
                            {
                                cancel = true;
                            }
                        });
                    });
                if create {
                    if let Some(mut created) = new_destination_draft.take() {
                        let id = next_destination_id(destinations);
                        created.id = id;
                        if created.name.trim().is_empty() {
                            created.name = format!("Destination {id}");
                        }
                        destinations.push(created);
                        source.destination_ids.push(id);
                        outcome.changed = true;
                    }
                } else if cancel {
                    *new_destination_draft = None;
                }
            }
        });
    });

    ui.add_space(10.0);

    // Section 3: Lokal inspelning (locked only while recording!)
    card_frame(palette, palette.card_border).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(strong_text(
                "Lokal inspelning",
                FONT_TITLE_S,
                palette.text_primary,
            ));
            crate::guide::tip(ui, "Välj var filen ska sparas. Mappknappen låter dig bläddra.");
            if recording {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    lock_badge(ui, "Låst under inspelning", palette);
                });
            }
        });
        ui.add_space(SPACE_S);

        let error_id = ui.make_persistent_id("open-recording-folder-error");
        let rec_field_w = if narrow {
            (ui.available_width() - 62.0).max(100.0)
        } else {
            (ui.available_width() - 168.0).max(140.0)
        };

        let render_folder_controls = |ui: &mut EguiUi,
                                      folder: &mut String,
                                      folder_label: egui::Id,
                                      outcome: &mut SourceSettingsOutcome| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let input = ui
                    .add(
                        egui::TextEdit::singleline(folder)
                            .margin(Margin::symmetric(6, 4))
                            .desired_width(rec_field_w),
                    )
                    .labelled_by(folder_label);
                if input.changed() {
                    outcome.changed = true;
                    ui.data_mut(|data| data.remove::<String>(error_id));
                }
                if icon_button(
                    ui,
                    Icon::Folder,
                    true,
                    palette.text_secondary,
                    false,
                    palette,
                    "Bläddra efter inspelningsmapp",
                )
                .clicked()
                {
                    let mut picker = rfd::FileDialog::new().set_title("Välj inspelningsmapp");
                    if !folder.is_empty() {
                        picker = picker.set_directory(folder.as_str());
                    }
                    if let Some(path) = picker.pick_folder() {
                        let selected = path.to_string_lossy();
                        if selected != folder.as_str() {
                            *folder = selected.into_owned();
                            outcome.changed = true;
                            ui.data_mut(|data| data.remove::<String>(error_id));
                        }
                    }
                }
                let empty = folder.trim().is_empty();
                if empty {
                    mark_invalid(ui, &input, palette);
                }
                if compact_button(ui, "Öppna mapp", !empty, None, palette).clicked() {
                    let error = open_recording_folder(folder).err();
                    ui.data_mut(|data| {
                        data.remove::<egui::Id>(error_id.with("dismissed"));
                        if let Some(error) = error {
                            data.insert_temp(error_id, error);
                        } else {
                            data.remove::<String>(error_id);
                        }
                    });
                }
            });
        };

        ui.add_enabled_ui(!recording, |ui| {
            let current_preset = FILENAME_FORMATS
                .iter()
                .find(|(_, template)| *template == source.recording.filename_format)
                .map_or("Anpassat", |(label, _)| *label);
            let preview = crate::output::preview_recording_filename(
                &source.recording.filename_format,
                &source.name,
                &source.recording.format,
            );

            if narrow {
                ui.spacing_mut().item_spacing.y = 6.0;
                let folder_label = ui.label("Mapp").id;
                render_folder_controls(ui, &mut source.recording.folder, folder_label, &mut outcome);
                let folder_error = ui.data(|data| data.get_temp::<String>(error_id));
                let hint = if source.recording.folder.trim().is_empty() {
                    Some("Välj en mapp för att kunna spela in.")
                } else {
                    folder_error.as_deref()
                };
                if ui.is_enabled() {
                    feedback::condition(ui, error_id.with("dismissed"), hint);
                }
                ui.label("Filnamnsformat");
                ComboBox::from_id_salt("filename-format")
                    .width((ui.available_width() - 8.0).max(120.0))
                    .truncate()
                    .selected_text(current_preset)
                    .show_ui(ui, |ui| {
                        for &(label, template) in FILENAME_FORMATS {
                            if ui
                                .selectable_label(source.recording.filename_format == template, label)
                                .clicked()
                            {
                                source.recording.filename_format = template.to_owned();
                                outcome.changed = true;
                            }
                        }
                    });

                let template_label = ui.label("Filnamnsmall").id;
                outcome.changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut source.recording.filename_format)
                            .margin(Margin::symmetric(6, 4))
                            .desired_width((ui.available_width() - 14.0).max(100.0)),
                    )
                    .labelled_by(template_label)
                    .on_hover_text(
                        "Platshållare: {namn}, {datum}, {tid}, {år}, {månad}, {dag}, {timme}, {minut}, {sekund}, {unix}",
                    )
                    .changed();
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("Exempel: {preview}"))
                            .size(FONT_CAPTION)
                            .color(palette.text_secondary),
                    )
                    .wrap(),
                );

                ui.label("Filformat");
                let was_lossy = bitrate_range(&source.recording.format).is_some();
                let format_changed = segmented_choice(
                    ui,
                    "format",
                    &mut source.recording.format,
                    &["WAV", "FLAC", "Opus", "MP3"],
                    palette,
                );
                outcome.changed |= format_changed;

                if let Some(range) = bitrate_range(&source.recording.format) {
                    ui.label("Bithastighet");
                    if format_changed {
                        source.recording.quality = if was_lossy {
                            source.recording.quality.clamp(*range.start(), *range.end())
                        } else if source.recording.format == "MP3" {
                            128
                        } else {
                            96
                        };
                    }
                    outcome.changed |= ui
                        .add(
                            egui::DragValue::new(&mut source.recording.quality)
                                .range(range)
                                .suffix(" kbit/s"),
                        )
                        .changed();
                } else {
                    ui.label("Bitdjup");
                    if format_changed && was_lossy {
                        source.recording.quality = 24;
                    }
                    outcome.changed |= segmented_u32(
                        ui,
                        "bit-depth",
                        &mut source.recording.quality,
                        &[(16, "16 bitar"), (24, "24 bitar")],
                        palette,
                    );
                }
            } else {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let folder_label = field_caption(ui, "Mapp", palette);
                    render_folder_controls(
                        ui,
                        &mut source.recording.folder,
                        folder_label,
                        &mut outcome,
                    );
                    let folder_error = ui.data(|data| data.get_temp::<String>(error_id));
                    let hint = if source.recording.folder.trim().is_empty() {
                        Some("Välj en mapp för att kunna spela in.")
                    } else {
                        folder_error.as_deref()
                    };
                    if ui.is_enabled() {
                        feedback::condition(ui, error_id.with("dismissed"), hint);
                    }
                });

                let (col_w, gap, spacing_x) = two_col_metrics(ui);
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    col_box(ui, col_w, spacing_x, |ui| {
                        field_caption(ui, "Filnamnsformat", palette);
                        let w = (ui.available_width() - 24.0).max(100.0);
                        ui.scope(|ui| {
                            ui.set_width(w + 24.0);
                            ComboBox::from_id_salt("filename-format")
                                .width(w)
                                .truncate()
                                .selected_text(current_preset)
                                .show_ui(ui, |ui| {
                                    for &(label, template) in FILENAME_FORMATS {
                                        if ui
                                            .selectable_label(
                                                source.recording.filename_format == template,
                                                label,
                                            )
                                            .clicked()
                                        {
                                            source.recording.filename_format = template.to_owned();
                                            outcome.changed = true;
                                        }
                                    }
                                });
                        });
                    });
                    col_box(ui, col_w, spacing_x, |ui| {
                        let template_label = field_caption(ui, "Filnamnsmall", palette);
                        outcome.changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut source.recording.filename_format)
                                    .margin(Margin::symmetric(6, 4))
                                    .desired_width((ui.available_width() - 14.0).max(100.0)),
                            )
                            .labelled_by(template_label)
                            .on_hover_text(
                                "Platshållare: {namn}, {datum}, {tid}, {år}, {månad}, {dag}, {timme}, {minut}, {sekund}, {unix}",
                            )
                            .changed();
                    });
                });

                ui.add(
                    egui::Label::new(
                        RichText::new(format!("Exempel: {preview}"))
                            .size(FONT_CAPTION)
                            .color(palette.text_secondary),
                    )
                    .wrap(),
                );

                let was_lossy = bitrate_range(&source.recording.format).is_some();
                let mut format_changed = false;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    col_box(ui, col_w, spacing_x, |ui| {
                        field_caption(ui, "Filformat", palette);
                        format_changed = segmented_choice(
                            ui,
                            "format",
                            &mut source.recording.format,
                            &["WAV", "FLAC", "Opus", "MP3"],
                            palette,
                        );
                        outcome.changed |= format_changed;
                    });
                    col_box(ui, col_w, spacing_x, |ui| {
                        if let Some(range) = bitrate_range(&source.recording.format) {
                            field_caption(ui, "Bithastighet", palette);
                            if format_changed {
                                source.recording.quality = if was_lossy {
                                    source.recording.quality.clamp(*range.start(), *range.end())
                                } else if source.recording.format == "MP3" {
                                    128
                                } else {
                                    96
                                };
                            }
                            outcome.changed |= ui
                                .add(
                                    egui::DragValue::new(&mut source.recording.quality)
                                        .range(range)
                                        .suffix(" kbit/s"),
                                )
                                .changed();
                        } else {
                            field_caption(ui, "Bitdjup", palette);
                            if format_changed && was_lossy {
                                source.recording.quality = 24;
                            }
                            outcome.changed |= segmented_u32(
                                ui,
                                "bit-depth",
                                &mut source.recording.quality,
                                &[(16, "16 bitar"), (24, "24 bitar")],
                                palette,
                            );
                        }
                    });
                });
            }
        });
    });

    outcome
}

fn password_visible(ctx: &Context, id: egui::Id) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let visible = data
            .get_temp::<u64>(id)
            .is_some_and(|last| pass.saturating_sub(last) <= 1);
        if visible {
            data.insert_temp(id, pass);
        }
        visible
    })
}

fn toggle_password(ctx: &Context, id: egui::Id, visible: bool) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        if visible {
            data.remove::<u64>(id);
        } else {
            data.insert_temp(id, pass);
        }
    });
}

fn destination_settings(
    ui: &mut EguiUi,
    destination: &mut Destination,
    narrow: bool,
    palette: Palette,
) -> bool {
    let mut changed = false;
    let show_id = ui.make_persistent_id("show-password");
    let show_password = password_visible(ui.ctx(), show_id);
    let single_column = narrow || ui.available_width() < 500.0;

    if single_column {
        ui.spacing_mut().item_spacing.y = 8.0;
        changed |= stacked_field(ui, "Namn", &mut destination.name, palette).changed();
        changed |= stacked_validated_field(
            ui,
            "Server-URL",
            &mut destination.server,
            server_hint,
            palette,
        );
        changed |= stacked_validated_field(
            ui,
            "Monteringspunkt",
            &mut destination.mount,
            mount_hint,
            palette,
        );
        changed |= stacked_validated_field(
            ui,
            "Användarnamn",
            &mut destination.user,
            crate::output::username_hint,
            palette,
        );

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let pw_label = field_caption(ui, "Lösenord", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut destination.password)
                            .password(!show_password)
                            .margin(Margin::symmetric(6, 4))
                            .desired_width((ui.available_width() - 56.0).max(80.0)),
                    )
                    .labelled_by(pw_label)
                    .changed();
                if icon_button(
                    ui,
                    if show_password {
                        Icon::EyeOff
                    } else {
                        Icon::Eye
                    },
                    true,
                    palette.text_secondary,
                    show_password,
                    palette,
                    if show_password {
                        "Dölj lösenord"
                    } else {
                        "Visa lösenord"
                    },
                )
                .clicked()
                {
                    toggle_password(ui.ctx(), show_id, show_password);
                }
            });
        });

        let mut codec_changed = false;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            field_caption(ui, "Kodek", palette);
            codec_changed = segmented_choice(
                ui,
                "codec",
                &mut destination.codec,
                &["Opus", "MP3"],
                palette,
            );
            changed |= codec_changed;
        });

        let range = bitrate_range(&destination.codec).unwrap_or(16..=510);
        if codec_changed {
            destination.bitrate = destination.bitrate.clamp(*range.start(), *range.end());
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            field_caption(ui, "Bithastighet", palette);
            changed |= ui
                .add(
                    egui::DragValue::new(&mut destination.bitrate)
                        .range(range)
                        .suffix(" kbit/s"),
                )
                .changed();
        });
        return changed;
    }

    // 2-column label-above-input layout (zero stagger, compact vertical rhythm)
    ui.spacing_mut().item_spacing.y = 10.0;
    let (col_w, gap, spacing_x) = two_col_metrics(ui);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        col_box(ui, col_w, spacing_x, |ui| {
            changed |= stacked_field(ui, "Namn", &mut destination.name, palette).changed();
        });
        col_box(ui, col_w, spacing_x, |ui| {
            changed |= stacked_validated_field(
                ui,
                "Server-URL",
                &mut destination.server,
                server_hint,
                palette,
            );
        });
    });
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        col_box(ui, col_w, spacing_x, |ui| {
            changed |= stacked_validated_field(
                ui,
                "Monteringspunkt",
                &mut destination.mount,
                mount_hint,
                palette,
            );
        });
        col_box(ui, col_w, spacing_x, |ui| {
            changed |= stacked_validated_field(
                ui,
                "Användarnamn",
                &mut destination.user,
                crate::output::username_hint,
                palette,
            );
        });
    });
    let mut codec_changed = false;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        col_box(ui, col_w, spacing_x, |ui| {
            let pw_label = field_caption(ui, "Lösenord", palette);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut destination.password)
                            .password(!show_password)
                            .margin(Margin::symmetric(6, 4))
                            .desired_width((ui.available_width() - 94.0).max(80.0)),
                    )
                    .labelled_by(pw_label)
                    .changed();
                let toggle_label = if show_password { "Dölj" } else { "Visa" };
                if action_button(
                    ui,
                    if show_password {
                        Icon::EyeOff
                    } else {
                        Icon::Eye
                    },
                    toggle_label,
                    true,
                    palette.text_secondary,
                    ButtonVariant::Neutral,
                    palette,
                )
                .clicked()
                {
                    toggle_password(ui.ctx(), show_id, show_password);
                }
            });
        });
        col_box(ui, col_w, spacing_x, |ui| {
            field_caption(ui, "Kodek", palette);
            codec_changed = segmented_choice(
                ui,
                "codec",
                &mut destination.codec,
                &["Opus", "MP3"],
                palette,
            );
            changed |= codec_changed;
        });
    });
    let range = bitrate_range(&destination.codec).unwrap_or(16..=510);
    if codec_changed {
        destination.bitrate = destination.bitrate.clamp(*range.start(), *range.end());
    }
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        col_box(ui, col_w, spacing_x, |ui| {
            field_caption(ui, "Bithastighet", palette);
            changed |= ui
                .add(
                    egui::DragValue::new(&mut destination.bitrate)
                        .range(range)
                        .suffix(" kbit/s"),
                )
                .changed();
        });
    });
    changed
}

fn bitrate_range(format: &str) -> Option<std::ops::RangeInclusive<u32>> {
    if format.eq_ignore_ascii_case("mp3") {
        Some(32..=320)
    } else if format.eq_ignore_ascii_case("opus") {
        Some(16..=510)
    } else {
        None
    }
}

fn open_recording_folder(folder: &str) -> Result<(), String> {
    let path = std::path::absolute(folder)
        .map_err(|_| "Mappsökvägen kunde inte läsas. Välj inspelningsmappen igen.")?;
    if !path.is_dir() {
        return Err("Mappen finns inte. Kontrollera sökvägen eller välj en befintlig mapp.".into());
    }
    let opener = if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|_| "Filhanteraren kunde inte öppnas. Öppna inspelningsmappen manuellt.".into())
}

fn server_hint(value: &str) -> Option<&'static str> {
    let value = value.trim();
    let Some((scheme, rest)) = value.split_once("://") else {
        return Some("Ange en serveradress som börjar med http:// eller https://.");
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Some("Använd http:// eller https:// för Icecast.");
    }
    if rest.is_empty() || rest.starts_with(['/', '\\', '?', '#']) {
        return Some("Ange serverns värdnamn eller IP-adress.");
    }
    if rest
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|authority| authority.ends_with(':'))
    {
        return Some("Ange en port efter kolon eller ta bort kolon.");
    }
    match url::Url::parse(value) {
        Ok(url) if url.port() == Some(0) => Some("Ange en port mellan 1 och 65535."),
        Err(url::ParseError::InvalidPort) => Some("Ange en port mellan 1 och 65535."),
        Ok(url) if url.has_host() => None,
        _ => Some("Kontrollera serverns värdnamn eller IP-adress."),
    }
}

fn mount_hint(value: &str) -> Option<&'static str> {
    let mount = value.trim();
    if !mount.starts_with('/') || mount.trim_start_matches('/').is_empty() {
        Some("Börja med / och ange ett namn, till exempel /live.")
    } else if mount.contains(['?', '#']) || mount.chars().any(char::is_control) {
        Some("Ta bort frågetecken, nummertecken och kontrolltecken.")
    } else {
        None
    }
}

fn mark_invalid(ui: &EguiUi, field: &egui::Response, palette: Palette) {
    ui.painter().rect_stroke(
        field.rect,
        CornerRadius::same(5),
        Stroke::new(1.5, palette.danger),
        egui::StrokeKind::Inside,
    );
}

fn field_caption(ui: &mut EguiUi, label: &str, palette: Palette) -> egui::Id {
    ui.label(
        RichText::new(label)
            .size(FONT_CAPTION)
            .color(palette.text_secondary),
    )
    .id
}

fn stacked_field(
    ui: &mut EguiUi,
    label: &str,
    value: &mut String,
    palette: Palette,
) -> egui::Response {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let label_id = field_caption(ui, label, palette);
        ui.add(
            egui::TextEdit::singleline(value)
                .margin(Margin::symmetric(6, 4))
                .desired_width((ui.available_width() - 14.0).max(80.0)),
        )
        .labelled_by(label_id)
    })
    .inner
}

fn stacked_validated_field(
    ui: &mut EguiUi,
    label: &str,
    value: &mut String,
    validate: fn(&str) -> Option<&'static str>,
    palette: Palette,
) -> bool {
    let input = stacked_field(ui, label, value, palette);
    let hint = validate(value);
    if hint.is_some() {
        mark_invalid(ui, &input, palette);
    }
    if ui.is_enabled() {
        feedback::condition(ui, ui.make_persistent_id(("field-error", label)), hint);
    }
    input.changed()
}

fn two_col_metrics(ui: &EguiUi) -> (f32, f32, f32) {
    let gap = 16.0;
    let col_w = ((ui.available_width() - gap) * 0.5).floor().max(110.0);
    (col_w, gap, ui.spacing().item_spacing.x)
}

fn col_box(
    ui: &mut EguiUi,
    col_w: f32,
    item_spacing_x: f32,
    add_contents: impl FnOnce(&mut EguiUi),
) {
    ui.allocate_ui_with_layout(Vec2::new(col_w, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_min_width(col_w);
        ui.spacing_mut().item_spacing.x = item_spacing_x;
        ui.spacing_mut().item_spacing.y = 4.0;
        add_contents(ui);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ui::click_button;

    #[test]
    fn stop_confirmation_fits_after_resizing_to_minimum_width() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let mut app = Ui::from_settings(Ok(Settings::default()), Engine::new(), None);
        app.confirm_stop = Some(StopConfirm::Close {
            discard_changes: false,
        });
        let id = egui::Id::new(("confirm-stop-modal", true));
        for width in [1200.0, 420.0] {
            let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 600.0));
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    app.stop_confirm_modal(ctx, Palette::for_theme(true))
                });
            }
            let dialog = ctx.memory(|m| m.area_rect(id)).unwrap();
            assert!(
                screen.contains_rect(dialog),
                "{dialog:?} outside {screen:?}"
            );
        }
    }

    #[test]
    fn active_window_close_requires_explicit_confirmation() {
        let ctx = Context::default();
        for (active, confirmed, blocked) in [
            (true, false, true),
            (true, false, true), // A second close request is still not confirmation.
            (true, true, false),
            (false, false, false),
        ] {
            let output = ctx.run(Default::default(), |ctx| {
                assert_eq!(guard_active_close(ctx, confirmed, active), blocked);
            });
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert_eq!(
                commands.contains(&egui::ViewportCommand::CancelClose),
                blocked
            );
        }
    }

    #[test]
    fn close_dialog_only_discards_unsaved_changes_when_confirmed() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let mut app = Ui::from_settings(
            Ok(Settings::default()),
            crate::engine::tests::test_engine(),
            None,
        );
        let palette = Palette::for_theme(false);
        for (discard_changes, button, expect_close, expect_dirty) in [
            (true, "Avbryt", false, true),
            (false, "Stäng", true, true),
            (true, "Stäng", true, false),
        ] {
            app.save_state = SaveState::Failed("disk full".into());
            app.confirm_stop = Some(StopConfirm::Close { discard_changes });
            app.close_confirmed = false;
            let output = click_button(&ctx, button, |ctx| app.stop_confirm_modal(ctx, palette));
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert_eq!(
                commands.contains(&egui::ViewportCommand::Close),
                expect_close
            );
            assert_eq!(app.close_confirmed, expect_close);
            assert_eq!(app.save_state.dirty(), expect_dirty);
            assert!(app.confirm_stop.is_none());
        }
    }

    #[test]
    fn new_source_shortcut_waits_until_dialogs_close() {
        let ctx = Context::default();
        let mut app = Ui::from_settings(
            Ok(Settings::default()),
            crate::engine::tests::test_engine(),
            None,
        );
        let press_shortcut = |app: &mut Ui| {
            let input = egui::RawInput {
                modifiers: NEW_SOURCE_SHORTCUT.modifiers,
                events: vec![egui::Event::Key {
                    key: NEW_SOURCE_SHORTCUT.logical_key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: NEW_SOURCE_SHORTCUT.modifiers,
                }],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| app.handle_shortcuts(ctx));
        };

        app.confirm_stop = Some(StopConfirm::Recordings);
        press_shortcut(&mut app);
        assert!(app.settings.sources.is_empty());

        app.confirm_stop = None;
        app.saved_recordings
            .push_back(crate::recording_dialog::SavedRecording::new(
                "Sal".into(),
                "Möte.wav".into(),
            ));
        press_shortcut(&mut app);
        assert!(app.settings.sources.is_empty());

        app.saved_recordings.clear();
        press_shortcut(&mut app);
        assert_eq!(app.settings.sources.len(), 1);
    }

    #[test]
    fn creating_a_destination_from_a_source_only_changes_routing_on_confirmation() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let mut source = Source {
            destination_ids: vec![2],
            ..Default::default()
        };
        let existing = Destination {
            id: 2,
            name: "Befintlig".into(),
            ..Default::default()
        };
        let mut destinations = vec![existing.clone()];
        let mut draft = None;
        let mut inline_edit = None;
        let mut tester = ConnectionTester::default();
        let streaming_dest_ids = HashSet::new();
        for (button, expect_changed) in [
            ("Skapa ny destination", false),
            ("Avbryt", false),
            ("Skapa ny destination", false),
            ("Skapa och välj", true),
        ] {
            let mut changed = false;
            click_button(&ctx, button, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    changed |= source_settings(
                        ui,
                        &mut source,
                        &mut destinations,
                        &mut draft,
                        &mut inline_edit,
                        &mut tester,
                        &[],
                        false,
                        false,
                        &streaming_dest_ids,
                        false,
                        Palette::for_theme(false),
                    )
                    .changed;
                });
            });
            assert_eq!(changed, expect_changed, "{button}");
            if !expect_changed {
                assert_eq!(destinations.as_slice(), std::slice::from_ref(&existing));
                assert_eq!(source.destination_ids, [2]);
            }
            match button {
                "Skapa ny destination" => draft.as_mut().unwrap().name = "Sändningssal".into(),
                "Avbryt" => assert!(draft.is_none()),
                _ => {}
            }
        }
        assert!(draft.is_none());
        assert_eq!(destinations.len(), 2);
        assert_eq!(destinations[0], existing);
        assert_eq!(destinations[1].name, "Sändningssal");
        assert_ne!(destinations[1].id, existing.id);
        assert_ne!(destinations[1].id, 0);
        assert_eq!(source.destination_ids, [existing.id, destinations[1].id]);
    }

    #[test]
    fn inline_destination_validation_identifies_fields_that_need_attention() {
        for server in [
            "https://radio.example",
            "http://[::1]:8000",
            "HTTP://localhost:65535",
        ] {
            assert!(server_hint(server).is_none(), "{server}");
        }
        for server in [
            "",
            "radio.example:8000",
            "ftp://radio.example",
            "https:///live",
            "http://:8000",
            "http://radio.example:bad",
            "http://localhost:",
            "http://localhost:65536",
            "http://localhost:0",
        ] {
            assert!(server_hint(server).is_some(), "{server}");
        }
        for mount in ["", "/", "live.opus", "/live?token", "/live#fragment"] {
            assert!(mount_hint(mount).is_some(), "{mount}");
        }
        assert!(mount_hint("/möten/live.opus").is_none());
    }

    #[test]
    fn source_settings_fit_narrow_window_with_long_names() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let name =
            "Microphone Array (Intel® Smart Sound Technology for Digital Microphones) ".repeat(3);
        let devices = vec![("mic".into(), name.clone(), 2)];
        let mut source = Source {
            device: "mic".into(),
            device_name: name.clone(),
            destination_ids: vec![1],
            ..Default::default()
        };
        let mut destinations = vec![Destination {
            id: 1,
            name,
            ..Default::default()
        }];
        source.recording.folder.clear();
        let mut draft = Some(Destination {
            server: String::new(),
            mount: String::new(),
            ..Default::default()
        });
        let mut inline_edit = Some(1);
        let mut tester = ConnectionTester::default();
        let streaming_dests = HashSet::new();
        for width in [1360.0, 820.0, 560.0, 420.0, 420.0] {
            let narrow = LayoutMode::from_width(width) == LayoutMode::Narrow;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 720.0))),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let right = ui.max_rect().right();
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            source_settings(
                                ui,
                                &mut source,
                                &mut destinations,
                                &mut draft,
                                &mut inline_edit,
                                &mut tester,
                                &devices,
                                false,
                                false,
                                &streaming_dests,
                                narrow,
                                Palette::for_theme(false),
                            );
                            assert!(
                                ui.min_rect().right() <= right + 1.0,
                                "settings at width {width} extend to {} beyond {right}",
                                ui.min_rect().right()
                            );
                        });
                });
            });
        }
    }

    #[test]
    fn strong_font_is_bound_on_first_frame() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        ctx.set_visuals_of(egui::Theme::Light, build_visuals(false));
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let palette = Palette::for_theme(ctx.style().visuals.dark_mode);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(strong_text("Mötessändare", 18.0, palette.text_primary));
                let _ = tab_button(ui, "Översikt", true, palette);
            });
        });
    }

    #[test]
    fn header_icon_buttons_are_ordered_aligned_and_non_overlapping() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(820.0, 720.0))),
            ..Default::default()
        };
        let mut rects = [Rect::NOTHING; 4];
        let mut count_rects = [Rect::NOTHING; 2];
        let mut ids = [egui::Id::NULL; 4];
        let _ = ctx.run(input, |ctx| {
            let palette = Palette::for_theme(true);
            egui::TopBottomPanel::top("header")
                .frame(
                    Frame::new()
                        .fill(palette.header_bg)
                        .inner_margin(Margin::symmetric(16, 10)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_min_height(36.0);
                        ui.label(strong_text("Mötessändare", 18.0, palette.text_primary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.set_height(36.0);
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let rec_count = header_count(ui, 1, 3, palette.record_accent, palette);
                            let (rec_start, rec_stop) =
                                header_pill(ui, "record-pill", palette, |ui| {
                                    let start = icon_button(
                                        ui,
                                        Icon::Record,
                                        true,
                                        palette.record_accent,
                                        false,
                                        palette,
                                        "Starta alla inspelningar",
                                    );
                                    let stop = icon_button(
                                        ui,
                                        Icon::RecordStop,
                                        true,
                                        palette.record_accent,
                                        false,
                                        palette,
                                        "Stoppa alla inspelningar",
                                    );
                                    (start, stop)
                                });
                            ui.add_space(6.0);
                            let str_count = header_count(ui, 2, 3, palette.stream_accent, palette);
                            let (str_start, str_stop) =
                                header_pill(ui, "stream-pill", palette, |ui| {
                                    let start = icon_button(
                                        ui,
                                        Icon::Broadcast,
                                        true,
                                        palette.stream_accent,
                                        false,
                                        palette,
                                        "Starta alla sändningar",
                                    );
                                    let stop = icon_button(
                                        ui,
                                        Icon::BroadcastStop,
                                        true,
                                        palette.stream_accent,
                                        false,
                                        palette,
                                        "Stoppa alla sändningar",
                                    );
                                    (start, stop)
                                });
                            rects = [str_start.rect, str_stop.rect, rec_start.rect, rec_stop.rect];
                            count_rects = [str_count.rect, rec_count.rect];
                            ids = [str_start.id, str_stop.id, rec_start.id, rec_stop.id];
                        });
                    });
                });
        });

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "header button IDs must be unique");
            }
        }

        let [str_start, str_stop, rec_start, rec_stop] = rects;
        let [str_count, rec_count] = count_rects;
        assert!(
            str_start.right() < str_stop.left(),
            "stream start ({str_start:?}) should be left of stream stop ({str_stop:?})"
        );
        assert!(
            str_stop.right() < str_count.left(),
            "stream pill ({str_stop:?}) should be left of stream count ({str_count:?})"
        );
        assert!(
            str_count.right() < rec_start.left(),
            "stream count ({str_count:?}) should be left of record pill ({rec_start:?})"
        );
        assert!(
            rec_start.right() < rec_stop.left(),
            "record start ({rec_start:?}) should be left of record stop ({rec_stop:?})"
        );
        assert!(
            rec_stop.right() < rec_count.left(),
            "record pill ({rec_stop:?}) should be left of record count ({rec_count:?})"
        );
        for other in [str_stop, rec_start, rec_stop] {
            assert!(
                (str_start.center().y - other.center().y).abs() < 0.5,
                "buttons should share vertical center: {str_start:?} vs {other:?}"
            );
        }
        for count_rect in [str_count, rec_count] {
            assert!(
                (str_start.center().y - count_rect.center().y).abs() < 1.5,
                "counts should align vertically with buttons: {str_start:?} vs {count_rect:?}"
            );
        }
        assert!(
            str_start.left() > 400.0,
            "bulk pills should be right-aligned, not stretched: {str_start:?}"
        );
    }

    #[test]
    fn compact_stereo_meter_uses_less_height_than_the_full_display() {
        let ctx = Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(560.0, 480.0))),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let palette = Palette::for_theme(false);
            egui::CentralPanel::default().show(ctx, |ui| {
                let levels = [
                    MeterLevel {
                        signal: 0.5,
                        held: 0.9,
                        clipped: false,
                    },
                    MeterLevel {
                        signal: 0.25,
                        held: 1.0,
                        clipped: true,
                    },
                ];
                let compact = ui
                    .scope(|ui| compact_meter(ui, &levels, palette))
                    .response
                    .rect;
                let full = ui
                    .scope(|ui| {
                        meter(ui, "Vänster", levels[0], false, palette);
                        meter(ui, "Höger", levels[1], true, palette);
                    })
                    .response
                    .rect;
                assert!(compact.height() <= full.height() * 0.5);
                assert!(compact.right() <= ui.clip_rect().right());
            });
        });
    }

    #[test]
    fn meter_gradient_and_mesh_clip_cleanly_across_dbfs_range() {
        let palette = Palette::for_theme(true);
        let db_to_signal = |db: f32| 10.0_f32.powf(db / 20.0);

        assert_eq!(meter_fraction(0.0), 0.0);
        assert_eq!(meter_fraction(f32::NAN), 0.0);
        assert_eq!(
            meter_gradient_color(db_to_meter_fraction(-60.0), palette),
            palette.meter_ok
        );
        assert_eq!(
            meter_gradient_color(db_to_meter_fraction(-24.0), palette),
            palette.meter_ok
        );
        let mid_ok_warn = meter_gradient_color(db_to_meter_fraction(-21.0), palette);
        assert_ne!(mid_ok_warn, palette.meter_ok);
        assert_ne!(mid_ok_warn, palette.meter_warn);
        assert_eq!(
            meter_gradient_color(db_to_meter_fraction(-18.0), palette),
            palette.meter_warn
        );
        assert_eq!(
            meter_gradient_color(db_to_meter_fraction(-6.0), palette),
            palette.meter_warn
        );
        let mid_warn_peak = meter_gradient_color(db_to_meter_fraction(-4.5), palette);
        assert_ne!(mid_warn_peak, palette.meter_warn);
        assert_ne!(mid_warn_peak, palette.meter_peak);
        assert_eq!(
            meter_gradient_color(db_to_meter_fraction(-3.0), palette),
            palette.meter_peak
        );
        assert_eq!(
            meter_gradient_color(db_to_meter_fraction(0.0), palette),
            palette.meter_peak
        );

        let expected = [
            (-60.0, 0.0),
            (-48.0, 0.2),
            (-36.0, 0.4),
            (-24.0, 0.6),
            (-18.0, 0.7),
            (-12.0, 0.8),
            (-6.0, 0.9),
            (-3.0, 0.95),
            (0.0, 1.0),
        ];
        for (db, frac) in expected {
            let actual = meter_fraction(db_to_signal(db));
            assert!(
                (actual - frac).abs() < 1e-4,
                "tick {db} dB expected fraction {frac}, got {actual}"
            );
        }

        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(300.0, 10.0));
        assert!(
            build_meter_gradient_mesh(rect, 0.0, palette)
                .vertices
                .is_empty()
        );

        let quiet_mesh = build_meter_gradient_mesh(rect, 0.5, palette);
        assert_eq!(quiet_mesh.vertices.len(), 4);
        assert!(
            quiet_mesh
                .vertices
                .iter()
                .all(|v| v.color == palette.meter_ok)
        );

        let mid_frac = db_to_meter_fraction(-12.0);
        let mid_mesh = build_meter_gradient_mesh(rect, mid_frac, palette);
        assert_eq!(mid_mesh.vertices.len(), 12);
        let max_x = mid_mesh
            .vertices
            .iter()
            .map(|v| v.pos.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_x - (rect.left() + rect.width() * mid_frac)).abs() < 1e-3);
        assert_eq!(mid_mesh.vertices.first().unwrap().color, palette.meter_ok);
        assert_eq!(
            mid_mesh.vertices[mid_mesh.vertices.len() - 2].color,
            palette.meter_warn
        );

        let full_mesh = build_meter_gradient_mesh(rect, 1.0, palette);
        assert_eq!(full_mesh.vertices.len(), 20);
        assert_eq!(full_mesh.vertices.first().unwrap().color, palette.meter_ok);
        assert_eq!(
            full_mesh.vertices[full_mesh.vertices.len() - 2].color,
            palette.meter_peak
        );
    }

    #[test]
    fn meter_scale_labels_never_overlap_at_min_or_narrow_widths() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.style_mut(|style| {
            style.spacing.item_spacing = Vec2::new(10.0, 8.0);
            style.spacing.interact_size.y = 28.0;
        });
        let font_id = FontId::proportional(9.5);
        let mut widths = Vec::new();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(560.0, 480.0))),
                ..Default::default()
            },
            |ctx| {
                let palette = Palette::for_theme(true);
                egui::CentralPanel::default()
                    .frame(Frame::new().inner_margin(Margin::symmetric(16, 12)))
                    .show(ctx, |ui| {
                        card_frame(palette, palette.card_border).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            meter(
                                ui,
                                "Vänster",
                                MeterLevel {
                                    signal: 0.5,
                                    ..Default::default()
                                },
                                false,
                                palette,
                            );
                            meter(
                                ui,
                                "Höger",
                                MeterLevel {
                                    signal: 0.9,
                                    ..Default::default()
                                },
                                true,
                                palette,
                            );
                        });
                        widths = METER_SCALE_LABELS
                            .iter()
                            .map(|&(db, text)| {
                                let w = ui
                                    .painter()
                                    .layout_no_wrap(
                                        text.to_owned(),
                                        font_id.clone(),
                                        Color32::PLACEHOLDER,
                                    )
                                    .size()
                                    .x;
                                (db, w)
                            })
                            .collect();
                    });
            },
        );
        assert!(
            output.shapes.len() > 10,
            "expected meter gradient, ticks, and scale shapes"
        );

        for bar_width in [100.0_f32, 160.0, 220.0, 320.0, 560.0] {
            let bar_left = 20.0;
            let min_gap = 4.0;
            let placed = layout_meter_scale_labels(bar_left, bar_width, &widths, min_gap);
            assert!(
                placed.len() >= 2,
                "expected at least endpoint labels at width {bar_width}"
            );
            for item in &placed {
                assert!(
                    item.label_x >= bar_left - 0.01
                        && item.label_x + item.width <= bar_left + bar_width + 0.01,
                    "label out of bounds at width {bar_width}: {}-{}",
                    item.label_x,
                    item.label_x + item.width
                );
            }
            for pair in placed.windows(2) {
                assert!(
                    pair[0].label_x + pair[0].width + min_gap <= pair[1].label_x + 0.01,
                    "labels overlap at width {bar_width}: {}..{} vs {}..{}",
                    pair[0].label_x,
                    pair[0].label_x + pair[0].width,
                    pair[1].label_x,
                    pair[1].label_x + pair[1].width
                );
            }
        }
    }

    #[test]
    fn autosave_waits_for_the_last_edit_and_keeps_failed_changes_dirty() {
        let start = Instant::now();
        let mut state = SaveState::Idle;
        assert!(!state.dirty());
        state.changed(start);
        let next_edit = start + Duration::from_millis(400);
        state.changed(next_edit);
        assert!(state.dirty());
        assert!(!state.due(start + AUTOSAVE_DELAY));
        assert!(state.due(next_edit + AUTOSAVE_DELAY));

        state = SaveState::Failed("disk full".into());
        assert!(state.dirty());
        assert!(!state.due(next_edit + AUTOSAVE_DELAY));
        state.changed(next_edit);
        assert!(state.due(next_edit + AUTOSAVE_DELAY));
    }

    #[test]
    fn ui_non_ascii_glyphs_exist_in_configured_fonts() {
        for use_system_fonts in [false, true] {
            let ctx = Context::default();
            if use_system_fonts {
                configure_fonts(&ctx);
            }
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                ctx.fonts(|fonts| {
                    let prop = FontId::proportional(13.5);
                    let mono = FontId::monospace(12.0);
                    let mut lock = fonts.lock();
                    for ch in ['Ö', 'ä', 'å', 'ö', '·', '–', '…'] {
                        assert!(
                            lock.fonts.font(&prop).has_glyph(ch),
                            "missing proportional glyph U+{:04X} ({ch:?}) (system_fonts={use_system_fonts})",
                            ch as u32
                        );
                    }
                    if use_system_fonts {
                        let strong = FontId::new(15.5, FontFamily::Name(STRONG_FONT.into()));
                        for ch in ['Ö', 'ä', 'å', 'ö'] {
                            assert!(
                                lock.fonts.font(&strong).has_glyph(ch),
                                "missing strong glyph U+{:04X} ({ch:?})",
                                ch as u32
                            );
                        }
                    }
                    for ch in ['−', '∞'] {
                        assert!(
                            lock.fonts.font(&mono).has_glyph(ch),
                            "missing monospace glyph U+{:04X} ({ch:?}) (system_fonts={use_system_fonts})",
                            ch as u32
                        );
                    }
                });
            });
        }
    }

    #[test]
    fn disabled_start_buttons_explain_why_and_trigger_hover() {
        let mut source = Source::default();
        assert_eq!(
            source_stream_disabled_reason(&source, 0, false),
            Some("Välj en ljudenhet i inställningarna")
        );
        assert_eq!(
            source_record_disabled_reason(&source, false),
            Some("Välj en ljudenhet i inställningarna")
        );

        source.device = "mic-1".to_owned();
        source.recording.folder.clear();
        assert_eq!(
            source_stream_disabled_reason(&source, 0, false),
            Some("Lägg till minst en destination")
        );
        assert_eq!(
            source_record_disabled_reason(&source, false),
            Some("Välj en inspelningsmapp")
        );

        source.recording.folder = "/tmp/rec".to_owned();
        assert_eq!(source_stream_disabled_reason(&source, 1, false), None);
        assert_eq!(source_record_disabled_reason(&source, false), None);

        // Verify disabled custom button (Sense::hover) registers hover and shows tooltip.
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        let mut button_rect = Rect::NOTHING;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 200.0))),
                ..Default::default()
            },
            |ctx| {
                let palette = Palette::for_theme(true);
                egui::CentralPanel::default().show(ctx, |ui| {
                    let resp = with_disabled_reason(
                        action_button(
                            ui,
                            Icon::Broadcast,
                            "Starta sändning",
                            false,
                            palette.stream_accent,
                            ButtonVariant::Neutral,
                            palette,
                        ),
                        false,
                        Some("Lägg till minst en destination"),
                    );
                    button_rect = resp.rect;
                });
            },
        );
        assert!(button_rect.width() > 0.0);

        let mut hovered = false;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 200.0))),
                events: vec![egui::Event::PointerMoved(button_rect.center())],
                time: Some(2.0),
                ..Default::default()
            },
            |ctx| {
                let palette = Palette::for_theme(true);
                egui::CentralPanel::default().show(ctx, |ui| {
                    let resp = with_disabled_reason(
                        action_button(
                            ui,
                            Icon::Broadcast,
                            "Starta sändning",
                            false,
                            palette.stream_accent,
                            ButtonVariant::Neutral,
                            palette,
                        ),
                        false,
                        Some("Lägg till minst en destination"),
                    );
                    hovered = resp.hovered();
                });
            },
        );
        assert!(hovered, "disabled custom button should still detect hover");
        assert!(
            output.shapes.len() > 3,
            "hovering disabled custom button should render tooltip shapes"
        );
    }

    #[test]
    fn password_visibility_toggle_is_scoped_per_destination_id() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        let mut dest_a = Destination {
            id: 1,
            password: "secret".to_owned(),
            ..Destination::default()
        };
        let mut dest_b = Destination {
            id: 2,
            password: "other".to_owned(),
            ..Destination::default()
        };

        let mut id_a = egui::Id::NULL;
        let mut id_b = egui::Id::NULL;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let palette = Palette::for_theme(true);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.push_id(dest_a.id, |ui| {
                    destination_settings(ui, &mut dest_a, false, palette);
                    id_a = ui.make_persistent_id("show-password");
                });
                ui.push_id(dest_b.id, |ui| {
                    destination_settings(ui, &mut dest_b, false, palette);
                    id_b = ui.make_persistent_id("show-password");
                });
            });
        });
        assert_ne!(id_a, id_b);
        let _ = ctx.run(Default::default(), |ctx| {
            assert!(!password_visible(ctx, id_a));
            toggle_password(ctx, id_a, false);
        });
        let _ = ctx.run(Default::default(), |ctx| {
            assert!(password_visible(ctx, id_a));
            assert!(!password_visible(ctx, id_b));
        });
        let _ = ctx.run(Default::default(), |_| {}); // The editor was closed.
        let _ = ctx.run(Default::default(), |ctx| {
            assert!(!password_visible(ctx, id_a))
        });
    }

    #[test]
    fn layout_mode_classifies_narrow_standard_and_wide_breakpoints() {
        assert_eq!(LayoutMode::from_width(420.0), LayoutMode::Narrow);
        assert_eq!(LayoutMode::from_width(559.0), LayoutMode::Narrow);
        assert_eq!(LayoutMode::from_width(560.0), LayoutMode::Standard);
        assert_eq!(LayoutMode::from_width(1099.0), LayoutMode::Standard);
        assert_eq!(LayoutMode::from_width(1100.0), LayoutMode::Wide);
        assert_eq!(LayoutMode::from_width(1360.0), LayoutMode::Wide);
    }

    #[test]
    fn narrow_source_card_fits_420px_without_overflow() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        let engine = Engine::new();
        let dest = Destination {
            id: 1,
            name: "Huvudsändning".into(),
            ..Destination::default()
        };
        let source = Source {
            id: 1,
            name: "Stora salen med ett extra långt namn som måste rymmas".into(),
            device: "mic-1".into(),
            right: Some(1),
            destination_ids: vec![1],
            ..Source::default()
        };
        engine.update(std::slice::from_ref(&source));

        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 600.0))),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            let palette = Palette::for_theme(true);
            egui::CentralPanel::default()
                .frame(Frame::new().inner_margin(Margin::symmetric(10, 12)))
                .show(ctx, |ui| {
                    let right = ui.max_rect().right();
                    source_card(
                        ui,
                        &engine,
                        &source,
                        std::slice::from_ref(&dest),
                        false,
                        false,
                        true,
                        false,
                        true,
                        palette,
                    );
                    assert!(
                        ui.min_rect().right() <= right + 1.0,
                        "narrow card extended to {} beyond {right}",
                        ui.min_rect().right()
                    );
                });
        });
    }

    #[test]
    fn undo_restores_deleted_source_and_destination_links() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let mut ui = Ui {
            saved_recordings: VecDeque::new(),
            engine: Engine::new(),
            settings: Settings {
                compact_overview: None,
                theme: ThemePreference::System,
                destinations: vec![Destination {
                    id: 1,
                    name: "Huvud".into(),
                    ..Destination::default()
                }],
                sources: vec![Source {
                    id: 1,
                    name: "Stora salen".into(),
                    destination_ids: vec![1],
                    ..Source::default()
                }],
            },
            selected_source: Some(1),
            view: View::Overview,
            listener_stats: ListenerStats::default(),
            connection_tester: ConnectionTester::default(),
            inline_edit_destination: Some(1),
            expanded_destinations: HashSet::from([1]),
            undo_toast: None,
            activity_log: VecDeque::new(),
            last_logged_issues: HashMap::new(),
            save_state: SaveState::Idle,
            recording_errors: VecDeque::new(),
            hidden_settings_error: None,
            new_destination_draft: None,
            confirm_stop: None,
            app_icon: None,
            close_blocked: false,
            close_confirmed: false,
            load_error: None,
        };

        ui.delete_destination(1);
        assert!(ui.settings.destinations.is_empty());
        assert!(ui.settings.sources[0].destination_ids.is_empty());
        assert!(ui.inline_edit_destination.is_none());
        assert!(ui.undo_toast.is_some());

        ui.undo_delete();
        assert_eq!(ui.settings.destinations.len(), 1);
        assert_eq!(ui.settings.sources[0].destination_ids, vec![1]);
        assert!(ui.undo_toast.is_none());

        ui.delete_source(0);
        assert!(ui.settings.sources.is_empty());
        assert!(ui.undo_toast.is_some());

        ui.undo_delete();
        assert_eq!(ui.settings.sources.len(), 1);
        assert_eq!(ui.settings.sources[0].name, "Stora salen");
    }

    #[test]
    fn undo_preserves_entities_created_after_the_deletion() {
        let mut app = Ui::from_settings(
            Ok(Settings {
                destinations: vec![Destination {
                    id: 1,
                    name: "Huvudsändning".into(),
                    ..Default::default()
                }],
                sources: vec![Source {
                    id: 1,
                    name: "Stora salen".into(),
                    destination_ids: vec![1],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            crate::engine::tests::test_engine(),
            None,
        );

        app.delete_destination(1);
        app.add_destination();
        let new_dest_id = app.settings.destinations[0].id;
        app.settings.sources[0].destination_ids.push(new_dest_id);
        app.undo_delete();
        let restored_dest = &app.settings.destinations[0];
        assert_eq!(restored_dest.name, "Huvudsändning");
        assert_ne!(restored_dest.id, new_dest_id);
        assert_eq!(app.settings.destinations[1].id, new_dest_id);
        assert_eq!(
            app.settings.sources[0].destination_ids,
            [restored_dest.id, new_dest_id]
        );

        app.delete_source(0);
        app.add_source();
        let new_source_id = app.settings.sources[0].id;
        app.new_destination_draft = Some(Destination::default());
        app.undo_delete();
        assert_eq!(app.settings.sources[0].name, "Stora salen");
        assert_ne!(app.settings.sources[0].id, new_source_id);
        assert_eq!(app.settings.sources[1].id, new_source_id);
        assert_eq!(app.selected_source, Some(new_source_id));
        assert!(app.new_destination_draft.is_some());
    }

    #[test]
    fn all_views_render_across_narrow_standard_and_wide_breakpoints() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        ctx.set_visuals_of(egui::Theme::Light, build_visuals(false));

        let mut app = Ui {
            saved_recordings: VecDeque::new(),
            engine: Engine::new(),
            settings: Settings {
                compact_overview: Some(false),
                theme: ThemePreference::Dark,
                destinations: vec![
                    Destination {
                        id: 1,
                        name: "Huvudsändning".into(),
                        ..Destination::default()
                    },
                    Destination {
                        id: 2,
                        name: "Reserv".into(),
                        ..Destination::default()
                    },
                ],
                sources: vec![
                    Source {
                        id: 1,
                        name: "Stora salen".into(),
                        device: "mic-1".into(),
                        right: Some(1),
                        destination_ids: vec![1, 2],
                        ..Source::default()
                    },
                    Source {
                        id: 2,
                        name: "Bönerum".into(),
                        device: "mic-2".into(),
                        destination_ids: vec![1],
                        ..Source::default()
                    },
                ],
            },
            selected_source: Some(1),
            view: View::Overview,
            listener_stats: ListenerStats::default(),
            connection_tester: ConnectionTester::default(),
            inline_edit_destination: Some(1),
            expanded_destinations: HashSet::from([2]),
            undo_toast: None,
            activity_log: VecDeque::from([ActivityEvent {
                time: "18:00".into(),
                message: "Sändning startad (Stora salen)".into(),
                severity: EventSeverity::Stream,
            }]),
            last_logged_issues: HashMap::new(),
            save_state: SaveState::Idle,
            recording_errors: VecDeque::from(["Reserv återansluter…".into()]),
            hidden_settings_error: None,
            new_destination_draft: None,
            confirm_stop: None,
            app_icon: None,
            close_blocked: false,
            close_confirmed: false,
            load_error: None,
        };

        for view in [View::Overview, View::SourceSettings, View::Destinations] {
            app.view = view;
            for width in [420.0_f32, 700.0, 1360.0] {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 760.0))),
                    ..Default::default()
                };
                let output = ctx.run(input, |ctx| {
                    let palette = Palette::for_theme(ctx.style().visuals.dark_mode);
                    let mode = LayoutMode::from_width(ctx.screen_rect().width());
                    egui::TopBottomPanel::top("header").show(ctx, |ui| {
                        app.header(ui, mode, palette);
                    });
                    if view == View::SourceSettings && width >= MASTER_DETAIL_LEFT_WIDTH {
                        app.source_master_rail(ctx, palette);
                    }
                    if view == View::SourceSettings && width >= MASTER_DETAIL_RIGHT_WIDTH {
                        app.source_live_control_panel(ctx, palette);
                    }
                    if view == View::Overview && mode == LayoutMode::Wide {
                        app.wide_live_panel(ctx, palette);
                    }
                });
                assert!(
                    !output.shapes.is_empty(),
                    "expected painted shapes at width {width}"
                );
            }
        }
    }

    #[test]
    fn centered_column_preserves_horizontal_item_spacing() {
        let ctx = Context::default();
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        });
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 400.0))),
            ..Default::default()
        };
        let mut inner_spacing_x = 0.0;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                centered_column(ui, FORM_MAX_WIDTH, |ui| {
                    inner_spacing_x = ui.spacing().item_spacing.x;
                });
            });
        });
        assert!((inner_spacing_x - 10.0).abs() < 0.01);
    }

    #[test]
    fn centered_column_preserves_vertical_height_for_listener_stats() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let destinations: Vec<Destination> = (1..=4)
            .map(|id| Destination {
                id,
                name: format!("Destination {id}"),
                server: "http://example.invalid:8000".into(),
                mount: format!("/mount-{id}"),
                ..Destination::default()
            })
            .collect();
        let mut stats = ListenerStats::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360.0, 760.0))),
            ..Default::default()
        };
        let mut stats_height = 0.0;
        let _ = ctx.run(input, |ctx| {
            let palette = Palette::for_theme(true);
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui
                    .scope(|ui| {
                        centered_column(ui, FORM_MAX_WIDTH, |ui| {
                            listener_stats_page(ui, palette, &destinations, &mut stats);
                        });
                    })
                    .response
                    .rect;
                stats_height = rect.height();
            });
        });
        assert!(
            stats_height > 250.0,
            "listener stats should use available height without clipping, got {stats_height}"
        );
    }

    #[test]
    fn centered_column_inside_scroll_area_expands_full_content_height() {
        let ctx = Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360.0, 400.0))),
            ..Default::default()
        };
        let mut content_height = 0.0;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let output = ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        centered_column(ui, FORM_MAX_WIDTH, |ui| {
                            for _ in 0..20 {
                                ui.allocate_exact_size(Vec2::new(400.0, 50.0), Sense::hover());
                            }
                        });
                    });
                content_height = output.content_size.y;
            });
        });
        assert!(
            content_height >= 1000.0,
            "centered_column inside ScrollArea should preserve full scrollable content height, got {content_height}"
        );
    }

    #[test]
    fn wrapping_dot_pill_fits_narrow_card_without_overflow() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360.0, 400.0))),
            ..Default::default()
        };
        let mut pill_right = 0.0;
        let mut clip_right = 0.0;
        let _ = ctx.run(input, |ctx| {
            let palette = Palette::for_theme(true);
            egui::CentralPanel::default().show(ctx, |ui| {
                clip_right = ui.clip_rect().right();
                let resp = ui.scope(|ui| {
                    wrapping_dot_pill(
                        ui,
                        "Inspelning · Kunde inte starta FFmpeg: program not found · Tolkbåset-2026-11-20-19-05.opus",
                        palette.warning,
                        palette.warn_pill_bg,
                        palette.warn_pill_border,
                        palette.warning,
                    );
                });
                pill_right = resp.response.rect.right();
            });
        });
        assert!(
            pill_right <= clip_right + 1.0,
            "wrapping_dot_pill should stay within narrow card width: pill_right={pill_right}, clip_right={clip_right}"
        );
    }

    #[test]
    fn narrow_compact_source_card_fits_420px_and_is_shorter() {
        let ctx = Context::default();
        configure_fonts(&ctx);
        let engine = Engine::new();
        let source = Source {
            id: 1,
            name: "Mycket långt ljudkällnamn för smalt fönster".into(),
            device: "Behringer UMC404HD".into(),
            right: Some(1),
            destination_ids: vec![1],
            ..Source::default()
        };
        let destinations = vec![Destination {
            id: 1,
            name: "Huvudström".into(),
            server: "http://example.invalid:8000".into(),
            mount: "/live".into(),
            ..Destination::default()
        }];
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 600.0))),
            ..Default::default()
        };
        let mut compact_rect = Rect::NOTHING;
        let mut regular_rect = Rect::NOTHING;
        let mut clip_right = 0.0;
        let _ = ctx.run(input, |ctx| {
            let palette = Palette::for_theme(true);
            egui::CentralPanel::default().show(ctx, |ui| {
                clip_right = ui.clip_rect().right();
                compact_rect = ui
                    .scope(|ui| {
                        let _ = source_card(
                            ui,
                            &engine,
                            &source,
                            &destinations,
                            true,
                            true,
                            true,
                            true,
                            true,
                            palette,
                        );
                    })
                    .response
                    .rect;
                regular_rect = ui
                    .scope(|ui| {
                        let _ = source_card(
                            ui,
                            &engine,
                            &source,
                            &destinations,
                            false,
                            false,
                            true,
                            false,
                            true,
                            palette,
                        );
                    })
                    .response
                    .rect;
            });
        });
        assert!(
            compact_rect.right() <= clip_right + 1.0,
            "narrow compact card should fit 420px: right={}, clip_right={clip_right}",
            compact_rect.right()
        );
        assert!(
            compact_rect.height() < regular_rect.height(),
            "compact narrow card ({}) should be shorter than regular narrow card ({})",
            compact_rect.height(),
            regular_rect.height()
        );
    }

    #[test]
    fn compact_overview_scales_to_multi_column_in_wide_and_standard() {
        // Fullscreen / wide central area (~1580 px): 3 compact columns vs 2 normal columns
        assert_eq!(overview_columns(1580.0, LayoutMode::Wide, false, 3), 2);
        assert_eq!(overview_columns(1580.0, LayoutMode::Wide, true, 3), 3);
        // Medium wide (~920 px): 2 compact columns instead of collapsing to 1
        assert_eq!(overview_columns(920.0, LayoutMode::Wide, true, 4), 2);
        // Standard (~760 px): 2 compact columns vs 1 normal column
        assert_eq!(overview_columns(760.0, LayoutMode::Standard, false, 4), 1);
        assert_eq!(overview_columns(760.0, LayoutMode::Standard, true, 4), 2);
        // Narrow (~400 px): always 1 column
        assert_eq!(overview_columns(400.0, LayoutMode::Narrow, true, 4), 1);
    }
}
