use crate::engine::MeterLevel;
use crate::listener_stats::{ListenerStats, MountStatus, StatsTarget};
use crate::{Destination, Engine, Settings, Source, settings_load, settings_save};
use eframe::egui::{
    self, Align, Color32, ColorImage, ComboBox, Context, CornerRadius, FontData, FontDefinitions,
    FontFamily, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense, Shadow,
    Shape, Stroke, TextureHandle, TextureOptions, Ui as EguiUi, Vec2,
};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant};

const STRONG_FONT: &str = "strong";
const SOURCE_SIDEBAR_THRESHOLD: f32 = 800.0;
const SAVED_FEEDBACK_DURATION: Duration = Duration::from_secs(2);
const AUTOSAVE_DELAY: Duration = Duration::from_millis(500);
const SAVE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

const FILENAME_FORMATS: &[(&str, &str)] = &[
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
    Broadcast,
    BroadcastStop,
    Record,
    RecordStop,
    Stop,
    Settings,
    Folder,
    Plus,
    Check,
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
    saved_recordings: std::collections::VecDeque<crate::recording_dialog::SavedRecording>,
    engine: Engine,
    settings: Settings,
    selected_source: Option<u64>,
    view: View,
    listener_stats: ListenerStats,
    save_state: SaveState,
    notice: Option<String>,
    remove_source: bool,
    remove_destination: Option<u64>,
    new_destination_draft: Option<Destination>,
    confirm_stop: Option<StopConfirm>,
    app_icon: Option<TextureHandle>,
    close_blocked: bool,
    load_blocked: bool,
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

        let (settings, notice) = match settings_load() {
            Ok(settings) => (settings, None),
            Err(err) => (
                Settings::default(),
                Some(format!("Inställningarna kunde inte läsas. {err}")),
            ),
        };
        let load_blocked = notice.is_some();
        let engine = Engine::new();
        engine.update(&settings.sources);
        let selected_source = settings.sources.first().map(|source| source.id);
        Self {
            engine,
            settings,
            selected_source,
            view: View::Overview,
            listener_stats: ListenerStats::default(),
            save_state: SaveState::Idle,
            saved_recordings: Default::default(),
            notice,
            remove_source: false,
            remove_destination: None,
            new_destination_draft: None,
            confirm_stop: None,
            app_icon,
            close_blocked: false,
            load_blocked,
        }
    }

    fn navigate(&mut self, view: View) {
        self.view = view;
        self.remove_source = false;
        self.remove_destination = None;
        self.new_destination_draft = None;
    }

    fn open_source_settings(&mut self, id: u64) {
        self.selected_source = Some(id);
        self.navigate(View::SourceSettings);
    }

    fn add_source(&mut self) {
        if self.load_blocked {
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
        if self.load_blocked {
            return;
        }
        let id = self.settings.next_destination_id();
        self.settings.destinations.push(Destination {
            id,
            name: format!("Destination {id}"),
            ..Destination::default()
        });
        self.navigate(View::Destinations);
        self.save_state.changed(Instant::now());
    }

    fn destination_streaming(&self, destination_id: u64) -> bool {
        self.settings.sources.iter().any(|source| {
            source.destination_ids.contains(&destination_id) && self.engine.streaming(source.id)
        })
    }

    fn source_sidebar(&mut self, ctx: &Context, palette: Palette) {
        if self.load_blocked
            || self.settings.sources.is_empty()
            || ctx.screen_rect().width() <= SOURCE_SIDEBAR_THRESHOLD
        {
            return;
        }

        let mut open_settings_id = None;
        egui::SidePanel::right("source_sidebar")
            .resizable(false)
            .exact_width(184.0)
            .frame(
                Frame::new()
                    .fill(palette.header_bg)
                    .inner_margin(Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(strong_text("Ljudkällor", 14.0, palette.text_primary));
                    crate::guide::tip(
                        ui,
                        "Välj en ljudkälla för att öppna dess inställningar. Du kan byta källa direkt här.",
                    );
                });
                ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for source in &self.settings.sources {
                        let name = display_name(source);
                        let selected = self.view == View::SourceSettings
                            && self.selected_source == Some(source.id);
                        let button = egui::Button::new(name)
                            .selected(selected)
                            .corner_radius(7)
                            .min_size(Vec2::new(ui.available_width(), 28.0))
                            .truncate();
                        if ui
                            .push_id(source.id, |ui| ui.add(button))
                            .inner
                            .clicked()
                            && !selected
                        {
                            open_settings_id = Some(source.id);
                        }
                    }
                });
            });
        if let Some(id) = open_settings_id {
            self.open_source_settings(id);
        }
    }

    fn save(&mut self) {
        match settings_save(&self.settings) {
            Ok(()) => {
                self.close_blocked = false;
                self.save_state = SaveState::Saved(Instant::now());
            }
            Err(err) => self.save_state = SaveState::Failed(err),
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
        let active_count = match kind {
            StopConfirm::Streams => self
                .settings
                .sources
                .iter()
                .filter(|source| self.engine.streaming(source.id))
                .count(),
            StopConfirm::Recordings => self
                .settings
                .sources
                .iter()
                .filter(|source| self.engine.recording(source.id))
                .count(),
        };
        if active_count == 0 {
            self.confirm_stop = None;
            return;
        }

        let (title, body) = match kind {
            StopConfirm::Streams => (
                "Stoppa alla sändningar?",
                if active_count == 1 {
                    "1 pågående sändning kommer att stoppas.".to_owned()
                } else {
                    format!("{active_count} pågående sändningar kommer att stoppas.")
                },
            ),
            StopConfirm::Recordings => (
                "Stoppa alla inspelningar?",
                if active_count == 1 {
                    "1 pågående inspelning kommer att stoppas.".to_owned()
                } else {
                    format!("{active_count} pågående inspelningar kommer att stoppas.")
                },
            ),
        };

        let mut confirmed = false;
        let mut cancelled = false;
        let modal = egui::Modal::new(egui::Id::new("confirm-stop-modal"))
            .frame(
                Frame::new()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0, palette.card_border))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(18, 16))
                    .shadow(palette.card_shadow),
            )
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                ui.label(strong_text(title, 16.0, palette.text_primary));
                ui.add_space(6.0);
                ui.label(RichText::new(&body).color(palette.text_secondary));
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if action_button(
                        ui,
                        Icon::Stop,
                        "Stoppa",
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
                StopConfirm::Streams => {
                    for source in &self.settings.sources {
                        if self.engine.streaming(source.id) {
                            self.engine
                                .set_stream(source, &self.settings.destinations, false);
                        }
                    }
                }
                StopConfirm::Recordings => {
                    for source in &self.settings.sources {
                        if self.engine.recording(source.id) {
                            self.engine.set_record(source, false);
                        }
                    }
                }
            }
            self.confirm_stop = None;
        } else if cancelled || modal.should_close() {
            self.confirm_stop = None;
        }
    }

    fn header(&mut self, ui: &mut EguiUi, palette: Palette) {
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

        ui.horizontal(|ui| {
            ui.set_min_height(36.0);
            ui.spacing_mut().item_spacing.x = if ui.available_width() < 550.0 {
                6.0
            } else {
                10.0
            };
            if let Some(icon) = &self.app_icon {
                ui.add(
                    egui::Image::new((icon.id(), Vec2::splat(26.0)))
                        .corner_radius(CornerRadius::same(6)),
                );
            }
            ui.label(strong_text("Mötessändare", 18.0, palette.text_primary));
            let help = crate::guide::enabled(ui.ctx());
            if icon_button(
                ui, Icon::Help, true, palette.text_secondary, help, palette,
                if help { "Stäng hjälptipsen" } else { "Visa hjälptips vid knappar och fält" },
            )
            .clicked()
            {
                crate::guide::set_enabled(ui.ctx(), !help);
            }
            let showing_stats = self.view == View::ListenerStats;
            if icon_button(
                ui,
                Icon::ListenerStats,
                !self.load_blocked,
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

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.set_height(36.0);
                ui.spacing_mut().item_spacing.x = 6.0;

                let show_group_names = ui.available_width() >= 550.0;
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
                    for source in &self.settings.sources {
                        if !source.device.is_empty()
                            && !source.recording.folder.trim().is_empty()
                            && !self.engine.recording(source.id)
                        {
                            self.engine.set_record(source, true);
                        }
                    }
                }
                if record_stop.clicked() {
                    self.confirm_stop = Some(StopConfirm::Recordings);
                }

                ui.add_space(6.0);

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
                    for source in &self.settings.sources {
                        if !source.device.is_empty()
                            && !source
                                .resolve_destinations(&self.settings.destinations)
                                .is_empty()
                            && !self.engine.streaming(source.id)
                        {
                            self.engine
                                .set_stream(source, &self.settings.destinations, true);
                        }
                    }
                }
                if ui.available_width() >= 100.0 {
                    ui.label(RichText::new("Alla ljudkällor").size(12.0).color(palette.text_secondary));
                }
                if stream_stop.clicked() {
                    self.confirm_stop = Some(StopConfirm::Streams);
                }
            });
        });

        if let Some(message) = self.notice.clone() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&message).color(palette.warning));
                if self.load_blocked
                    && text_button(ui, "Läs in igen", true, ButtonVariant::Neutral, palette)
                        .clicked()
                {
                    match settings_load() {
                        Ok(settings) => {
                            self.selected_source = settings.sources.first().map(|source| source.id);
                            self.settings = settings;
                            self.engine.update(&self.settings.sources);
                            self.load_blocked = false;
                            self.notice = None;
                        }
                        Err(err) => {
                            self.notice =
                                Some(format!("Inställningarna kunde inte läsas. {err}"));
                        }
                    }
                }
            });
        }
        if let SaveState::Failed(error) = &self.save_state {
            let message = format!("Inställningarna kunde inte sparas. {error}");
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(palette.warning, message);
                if text_button(ui, "Försök igen", true, ButtonVariant::Neutral, palette).clicked() {
                    self.save();
                }
                if self.close_blocked
                    && text_button(
                        ui,
                        "Stäng utan att spara",
                        true,
                        ButtonVariant::Neutral,
                        palette,
                    )
                    .clicked()
                {
                    self.save_state = SaveState::Idle;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let palette = Palette::for_theme(ctx.style().visuals.dark_mode);

        if !self.load_blocked
            && ctx.input_mut(|input| input.consume_shortcut(&SAVE_SHORTCUT))
            && self.save_state.dirty()
        {
            self.save();
        }

        egui::TopBottomPanel::top("header")
            .frame(
                Frame::new()
                    .fill(palette.header_bg)
                    .inner_margin(Margin::symmetric(16, 10))
                    .stroke(Stroke::new(1.0, palette.card_border)),
            )
            .show(ctx, |ui| {
                self.header(ui, palette);
            });

        self.source_sidebar(ctx, palette);

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(palette.canvas_bg)
                    .inner_margin(Margin::symmetric(14, 12)),
            )
            .show(ctx, |ui| {
                if self.load_blocked {
                    return;
                }

                // Toolbar / Navigation bar: tabs on the left, actions on the right
                ui.horizontal(|ui| {
                    ui.set_min_height(30.0);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if tab_button(
                        ui,
                        "Översikt",
                        self.view == View::Overview || self.view == View::SourceSettings,
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

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_height(30.0);
                        ui.spacing_mut().item_spacing.x = 8.0;

                        if matches!(self.view, View::Overview | View::SourceSettings) {
                            let label = "Lägg till ljudkälla";
                            // Leave room for save feedback without moving the add button.
                            let add_source = if show_compact && ui.available_width() < 235.0 {
                                icon_button(
                                    ui,
                                    Icon::Plus,
                                    true,
                                    palette.text_primary,
                                    false,
                                    palette,
                                    label,
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
                            };
                            if add_source.clicked() {
                                self.add_source();
                            }
                        }

                        match self.save_state {
                            SaveState::Pending(_) => {
                                ui.weak("Sparar snart…");
                            }
                            SaveState::Saved(at) if at.elapsed() < SAVED_FEEDBACK_DURATION => {
                                saved_button(ui, "Sparat", palette);
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
                        listener_stats_page(
                            ui,
                            palette,
                            &self.settings.destinations,
                            &mut self.listener_stats,
                        );
                    }
                    View::Overview => {
                        if self.settings.sources.is_empty() {
                            empty_sources_view(ui, palette, || self.add_source());
                            return;
                        }
                        let mut open_settings_id = None;
                        let compact = self
                            .settings
                            .compact_overview
                            .unwrap_or(self.settings.sources.len() >= 4);
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 14.0;
                                for source in &self.settings.sources {
                                    ui.push_id(source.id, |ui| {
                                        let clicked_settings = source_card(
                                            ui,
                                            &self.engine,
                                            source,
                                            &self.settings.destinations,
                                            self.engine.streaming(source.id),
                                            self.engine.recording(source.id),
                                            true,
                                            compact,
                                            palette,
                                        );
                                        if clicked_settings {
                                            open_settings_id = Some(source.id);
                                        }
                                    });
                                }
                            });
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
                        let mut removed = false;
                        let mut outcome = SourceSettingsOutcome::default();
                        let devices = self.engine.devices();
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
                                source_card(
                                    ui,
                                    &self.engine,
                                    source,
                                    destinations,
                                    streaming,
                                    recording,
                                    false,
                                    false,
                                    palette,
                                );
                                if active {
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new(
                                            "Stoppa sändning och inspelning för att ändra inställningarna.",
                                        )
                                        .color(palette.warning),
                                    );
                                }
                                ui.add_space(10.0);
                                ui.add_enabled_ui(!active, |ui| {
                                    outcome = ui
                                        .push_id(source.id, |ui| {
                                            source_settings(
                                                ui,
                                                source,
                                                destinations,
                                                &mut self.new_destination_draft,
                                                &devices,
                                                palette,
                                            )
                                        })
                                        .inner;
                                    ui.add_space(10.0);
                                    card_frame(palette, palette.card_border).show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        if self.remove_source {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    RichText::new("Ta bort den här ljudkällan?")
                                                        .color(palette.danger),
                                                );
                                                removed = action_button(
                                                    ui,
                                                    None,
                                                    "Ta bort",
                                                    true,
                                                    Color32::WHITE,
                                                    ButtonVariant::Danger,
                                                    palette,
                                                )
                                                .clicked();
                                                if text_button(
                                                    ui,
                                                    "Avbryt",
                                                    true,
                                                    ButtonVariant::Neutral,
                                                    palette,
                                                )
                                                .clicked()
                                                {
                                                    self.remove_source = false;
                                                }
                                            });
                                        } else if neutral_button_colored(
                                            ui,
                                            "Ta bort ljudkälla",
                                            true,
                                            palette.danger,
                                            palette,
                                        )
                                        .clicked()
                                        {
                                            self.remove_source = true;
                                        }
                                    });
                                });
                            });
                        if outcome.open_destinations {
                            self.navigate(View::Destinations);
                        }
                        if removed {
                            self.settings.sources.remove(index);
                            self.selected_source = self
                                .settings
                                .sources
                                .get(index)
                                .or_else(|| self.settings.sources.last())
                                .map(|source| source.id);
                            self.navigate(View::Overview);
                        }
                        if outcome.changed || removed {
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
                                    18.0,
                                    palette.text_primary,
                                ));
                                ui.add_space(4.0);
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
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(strong_text(
                                        "Sparade destinationer",
                                        16.5,
                                        palette.text_primary,
                                    ));
                                    crate::guide::tip(ui, "Här sparar du vart ljudet ska skickas. Be den som sköter sändningen om adress och lösenord.");
                                    ui.with_layout(
                                        Layout::right_to_left(Align::Center),
                                        |ui| {
                                            if action_button(
                                                ui,
                                                Icon::Plus,
                                                "Lägg till destination",
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
                                ui.add_space(8.0);
                                for index in 0..self.settings.destinations.len() {
                                    let dest_id = self.settings.destinations[index].id;
                                    let used_by: Vec<String> = self
                                        .settings
                                        .sources
                                        .iter()
                                        .filter(|source| source.destination_ids.contains(&dest_id))
                                        .map(|source| display_name(source).to_owned())
                                        .collect();
                                    let in_use_streaming = self.destination_streaming(dest_id);
                                    ui.push_id(dest_id, |ui| {
                                        card_frame(palette, palette.card_border).show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            let dest = &mut self.settings.destinations[index];
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(strong_text(
                                                    dest.display_name(),
                                                    15.5,
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
                                            });
                                            if in_use_streaming {
                                                ui.label(
                                                    RichText::new(
                                                        "Stoppa pågående sändningar som använder den här destinationen för att ändra inställningarna.",
                                                    )
                                                    .color(palette.warning),
                                                );
                                            }
                                            ui.add_space(4.0);
                                            ui.add_enabled_ui(!in_use_streaming, |ui| {
                                                changed |= destination_settings(ui, dest, palette);
                                                ui.add_space(6.0);
                                                if self.remove_destination == Some(dest_id) {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let prompt = match used_by.len() {
                                                            0 => "Ta bort den här destinationen?"
                                                                .to_owned(),
                                                            1 => "Ta bort den här destinationen? Den tas även bort från 1 ljudkälla."
                                                                .to_owned(),
                                                            count => format!(
                                                                "Ta bort den här destinationen? Den tas även bort från {count} ljudkällor."
                                                            ),
                                                        };
                                                        ui.label(
                                                            RichText::new(prompt)
                                                                .color(palette.danger),
                                                        );
                                                        if action_button(
                                                            ui,
                                                            None,
                                                            "Ta bort",
                                                            true,
                                                            Color32::WHITE,
                                                            ButtonVariant::Danger,
                                                            palette,
                                                        )
                                                        .clicked()
                                                        {
                                                            remove_id = Some(dest_id);
                                                        }
                                                        if text_button(
                                                            ui,
                                                            "Avbryt",
                                                            true,
                                                            ButtonVariant::Neutral,
                                                            palette,
                                                        )
                                                        .clicked()
                                                        {
                                                            self.remove_destination = None;
                                                        }
                                                    });
                                                } else if compact_button(
                                                    ui,
                                                    "Ta bort destination",
                                                    true,
                                                    Some(palette.danger),
                                                    palette,
                                                )
                                                .clicked()
                                                {
                                                    self.remove_destination = Some(dest_id);
                                                }
                                            });
                                        });
                                    });
                                    ui.add_space(10.0);
                                }
                            });
                        if add_new {
                            self.add_destination();
                        }
                        if let Some(dest_id) = remove_id {
                            self.settings
                                .destinations
                                .retain(|dest| dest.id != dest_id);
                            for source in &mut self.settings.sources {
                                source.destination_ids.retain(|id| *id != dest_id);
                            }
                            self.remove_destination = None;
                            changed = true;
                        }
                        if changed {
                            self.save_state.changed(Instant::now());
                            self.engine.update(&self.settings.sources);
                        }
                    }
                }
            });

        for (source, result) in self.engine.completed_recordings() {
            match result {
                Ok(path) => self.saved_recordings.push_back(crate::recording_dialog::SavedRecording::new(source, path)),
                Err(error) => self.notice = Some(format!("Kunde inte slutföra inspelningen för {source}. {error}")),
            }
        }
        if self.confirm_stop.is_some() {
            self.stop_confirm_modal(ctx, palette);
        } else if let Some(recording) = self.saved_recordings.front_mut()
            && recording.show(ctx)
        {
            self.saved_recordings.pop_front();
        }

        let closing = ctx.input(|input| input.viewport().close_requested());
        let now = Instant::now();
        if self.save_state.due(now) || (closing && self.save_state.dirty()) {
            self.save();
            if closing && self.save_state.dirty() {
                self.close_blocked = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        let mut repaint_after = self.repaint_interval();
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

fn listener_stats_page(
    ui: &mut EguiUi,
    palette: Palette,
    destinations: &[Destination],
    stats: &mut ListenerStats,
) {
    let targets: Vec<_> = destinations.iter().map(StatsTarget::parse).collect();
    let servers: std::collections::HashSet<_> = targets
        .iter()
        .filter_map(|target| target.as_ref().ok().map(|target| target.server.as_str()))
        .collect();
    let refresh = ui
        .horizontal(|ui| {
            ui.label(strong_text("Lyssnarstatistik", 18.0, palette.text_primary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let refreshing = stats.refreshing();
                let clicked = text_button(
                    ui,
                    "Uppdatera",
                    !servers.is_empty() && !refreshing,
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
    stats.update(ui.ctx(), servers, refresh);

    if destinations.is_empty() {
        ui.weak("Lägg till en Icecast-destination under Destinationer för att se lyssnarstatistik.");
        return;
    }

    ScrollArea::vertical().id_salt("listener_stats").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        for (destination, target) in destinations.iter().zip(&targets) {
            card_frame(palette, palette.card_border).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(strong_text(destination.display_name(), 15.0, palette.text_primary));
                let target = match target {
                    Ok(target) => target,
                    Err(error) => {
                        ui.colored_label(palette.warning, *error);
                        return;
                    }
                };
                ui.weak(format!("{}{}", target.server, target.mount));
                match stats.status(target) {
                    MountStatus::Loading => {
                        ui.weak("Hämtar statistik…");
                    }
                    MountStatus::Unavailable(error) => {
                        ui.colored_label(palette.warning, error);
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
        fonts
            .font_data
            .insert("ui-regular".to_owned(), Arc::new(FontData::from_owned(bytes)));
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
        fonts
            .font_data
            .insert("ui-strong".to_owned(), Arc::new(FontData::from_owned(bytes)));
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
    let image = ColorImage::from_rgba_unmultiplied(
        [icon.width as usize, icon.height as usize],
        &icon.rgba,
    );
    Some(ctx.load_texture("app-icon", image, TextureOptions::LINEAR))
}

fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;

    match icon {
        Icon::Help => {
            painter.circle_stroke(c, r * 0.93, Stroke::new(1.3, color));
            painter.text(c, egui::Align2::CENTER_CENTER, "?", FontId::proportional(r * 1.55), color);
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
        response.on_hover_text(reason).on_disabled_hover_text(reason)
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
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, tooltip));

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
    let is_saved = matches!(spec.variant, ButtonVariant::Saved);
    let is_enabled = spec.enabled && ui.is_enabled() && !is_saved;
    let font_id = FontId::proportional(spec.font_size);
    let text_galley = ui
        .painter()
        .layout_no_wrap(spec.label.to_owned(), font_id, Color32::PLACEHOLDER);
    let content_width = if spec.icon.is_some() {
        BUTTON_ICON_SIZE + BUTTON_ICON_GAP + text_galley.size().x
    } else {
        text_galley.size().x
    };
    let width = content_width + spec.pad_x * 2.0;

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
                (bg, palette.stream_active_border, Color32::WHITE, Color32::WHITE)
            }
            ButtonVariant::RecordActive | ButtonVariant::Danger => {
                let bg = if response.hovered() {
                    palette.record_active_hover
                } else {
                    palette.record_active_bg
                };
                (bg, palette.record_active_border, Color32::WHITE, Color32::WHITE)
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
        let text_pos = Pos2::new(
            content_left,
            rect.center().y - text_galley.size().y * 0.5,
        );
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
        (Color32::TRANSPARENT, Color32::TRANSPARENT, palette.text_secondary)
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
) {
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(text.into().size(12.0).color(fg));
        });
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
        crate::guide::tip(ui, "Klicka på Lägg till ljudkälla. Välj sedan din mikrofon i listan.");
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

fn display_name(source: &Source) -> &str {
    if source.name.trim().is_empty() {
        "Namnlös ljudkälla"
    } else {
        &source.name
    }
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
    let server = dest.server.trim().trim_end_matches('/');
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
    palette: Palette,
) -> bool {
    let mut open_settings = false;
    let live = engine.live_status(source.id);
    let border_color = if recording {
        palette.record_active_border
    } else if streaming {
        palette.stream_active_border
    } else {
        palette.card_border
    };

    let margin = if compact {
        Margin::symmetric(10, 6)
    } else {
        Margin::symmetric(14, 9)
    };
    let frame = card_frame(palette, border_color).inner_margin(margin);
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = if compact { 3.0 } else { 5.0 };

        // Card header: title + active badges on left, settings button on right
        ui.horizontal_wrapped(|ui| {
            ui.label(strong_text(
                display_name(source),
                16.5,
                palette.text_primary,
            ));
            crate::guide::tip(ui, "Starta inspelning sparar ljudet på datorn. Starta sändning låter andra lyssna direkt.");
            if streaming {
                status_pill(
                    ui,
                    if live.connected > 0 {
                        "Sänder"
                    } else {
                        "Sändning begärd"
                    },
                    palette.stream_pill_bg,
                    palette.stream_active_border,
                    palette.stream_accent,
                );
            }
            if recording {
                status_pill(
                    ui,
                    RichText::new(format!(
                        "Spelar in · {:02}:{:02}:{:02}",
                        live.elapsed.as_secs() / 3600,
                        live.elapsed.as_secs() / 60 % 60,
                        live.elapsed.as_secs() % 60,
                    ))
                    .monospace(),
                    palette.record_pill_bg,
                    palette.record_active_border,
                    palette.record_accent,
                );
            }

            if compact {
                source_output_controls(
                    ui,
                    engine,
                    source,
                    destinations,
                    streaming,
                    recording,
                    true,
                    palette,
                );
                open_settings = icon_button(
                    ui,
                    Icon::Settings,
                    true,
                    palette.text_secondary,
                    false,
                    palette,
                    "Inställningar",
                )
                .clicked();
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
                        open_settings = true;
                    }
                });
            }
        });

        if !compact {
            let (sep_rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, palette.card_divider);
        }

        let present = engine.present(source.id);

        if !present {
            let message = if source.device.is_empty() {
                "Välj en ljudingång i inställningarna."
            } else if streaming && recording {
                "Ljudingången saknas – sändning och inspelning får tyst signal."
            } else if streaming {
                "Ljudingången saknas – sändningen får tyst signal."
            } else if recording {
                "Ljudingången saknas – inspelningen får tyst signal."
            } else {
                "Väntar på ljudingången."
            };
            ui.label(RichText::new(message).strong().color(palette.warning));
        }
        if live.total > 0 {
            ui.label(format!(
                "{} av {} sändningsmål anslutna",
                live.connected, live.total
            ))
            .on_hover_text(live.targets.join("\n"));
            for detail in &live.issues {
                ui.colored_label(palette.warning, detail);
            }
        }
        if !live.record_detail.is_empty() {
            let color = if recording {
                palette.text_secondary
            } else {
                palette.warning
            };
            ui.label(RichText::new(&live.record_detail).size(12.5).color(color));
        }

        let mut levels = engine.levels(source.id);
        if !present {
            for level in &mut levels {
                level.signal = 0.0;
            }
        }
        let stereo = source.right.is_some();
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
                    engine,
                    source,
                    destinations,
                    streaming,
                    recording,
                    false,
                    palette,
                );
            });
        }
    });
    open_settings
}

#[allow(clippy::too_many_arguments)]
fn source_output_controls(
    ui: &mut EguiUi,
    engine: &Engine,
    source: &Source,
    destinations: &[Destination],
    streaming: bool,
    recording: bool,
    compact: bool,
    palette: Palette,
) {
    let count = source.resolve_destinations(destinations).len();
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
                false,
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
            ).on_hover_text("De som lyssnar kan höra ljudet medan du sänder.")
        },
        stream_enabled,
        stream_reason,
    )
    .clicked()
    {
        engine.set_stream(source, destinations, !streaming);
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
            ).on_hover_text("Ljudet sparas i en fil på din dator.")
        },
        record_enabled,
        record_reason,
    )
    .clicked()
    {
        engine.set_record(source, !recording);
    }

    if !compact {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            status_pill(
                ui,
                if count == 1 {
                    "1 destination".to_owned()
                } else {
                    format!("{count} destinationer")
                },
                palette.pill_bg,
                palette.pill_border,
                palette.text_secondary,
            );
        });
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
            label_x < other.label_x + other.width + min_gap
                && other.label_x < label_right + min_gap
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
        .map(|&(_, text)| painter.layout_no_wrap(text.to_owned(), font_id.clone(), Color32::PLACEHOLDER))
        .collect();
    let candidates: Vec<(f32, f32)> = METER_SCALE_LABELS
        .iter()
        .zip(&galleys)
        .map(|(&(db, _), galley)| (db, galley.size().x))
        .collect();

    let placed = layout_meter_scale_labels(bar_rect.left(), bar_rect.width(), &candidates, 4.0);
    for item in placed {
        let tick_x = item.tick_x.clamp(bar_rect.left() + 0.5, bar_rect.right() - 0.5);
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
    open_destinations: bool,
}

fn source_settings(
    ui: &mut EguiUi,
    source: &mut Source,
    destinations: &mut Vec<Destination>,
    new_destination_draft: &mut Option<Destination>,
    devices: &[(String, String, usize)],
    palette: Palette,
) -> SourceSettingsOutcome {
    let mut outcome = SourceSettingsOutcome::default();

    // Section 1: Ljudingång
    card_frame(palette, palette.card_border).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(strong_text("Ljudingång", 15.5, palette.text_primary));
            crate::guide::tip(ui, "Välj din mikrofon och prata en stund. Stapeln rör sig när ljudet kommer in.");
        });
        ui.add_space(4.0);
        egui::Grid::new("input-fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                outcome.changed |= field(ui, "Namn", &mut source.name).changed();
                ui.label("Ljudenhet");
                let mut device = source.device.clone();
                let selected_name = devices
                    .iter()
                    .find(|(id, _, _)| id == &device)
                    .map(|(_, name, _)| name.as_str())
                    .unwrap_or_else(|| {
                        if source.device_name.is_empty() {
                            "Välj ljudenhet"
                        } else {
                            &source.device_name
                        }
                    });
                ui.scope(|ui| {
                    ui.set_width(290.0);
                    ComboBox::from_id_salt("device")
                        .width(260.0)
                        .truncate()
                        .selected_text(selected_name)
                        .show_ui(ui, |ui| {
                            if devices.is_empty() {
                                ui.label(
                                    RichText::new("Inga ljudenheter tillgängliga")
                                        .color(palette.text_secondary),
                                );
                            }
                            for (id, name, _) in devices {
                                ui.selectable_value(&mut device, id.clone(), name)
                                    .on_hover_text(name);
                            }
                        })
                        .response
                        .on_hover_text(selected_name);
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
                ui.end_row();
                let count = devices
                    .iter()
                    .find(|(id, _, _)| id == &source.device)
                    .map_or(0, |(_, _, count)| *count);
                ui.label(if source.right.is_some() {
                    "Vänster kanal"
                } else {
                    "Monokanal"
                });
                let left = source.left;
                let old_right = source.right;
                ComboBox::from_id_salt("left")
                    .selected_text((left + 1).to_string())
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
                ui.end_row();
                ui.label("Höger kanal");
                ComboBox::from_id_salt("right")
                    .selected_text(source.right.map_or_else(
                        || "Ingen (mono)".to_owned(),
                        |right| (right + 1).to_string(),
                    ))
                    .show_ui(ui, |ui| {
                        outcome.changed |= ui
                            .selectable_value(&mut source.right, None, "Ingen (mono)")
                            .changed();
                        for channel in (0..count).filter(|channel| *channel != source.left) {
                            outcome.changed |= ui
                                .selectable_value(
                                    &mut source.right,
                                    Some(channel),
                                    (channel + 1).to_string(),
                                )
                                .changed();
                        }
                    });
                ui.end_row();
            });
    });

    ui.add_space(10.0);

    // Section 2: Sändningsdestinationer
    card_frame(palette, palette.card_border).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(strong_text("Sändningsdestinationer", 15.5, palette.text_primary));
            crate::guide::tip(ui, "Välj vart ljudet ska sändas. Du kan hoppa över det här om du bara vill spela in.");
        });
        ui.add_space(4.0);
        if source.destination_ids.is_empty() {
            ui.label(
                RichText::new("Ingen destination vald för den här ljudkällan.")
                    .color(palette.text_secondary),
            );
        }
        let mut remove_slot = None;
        let mut replace_slot: Option<(usize, u64)> = None;
        for (slot_index, &dest_id) in source.destination_ids.iter().enumerate() {
            let Some(current) = destinations.iter().find(|dest| dest.id == dest_id) else {
                continue;
            };
            ui.push_id(("dest-slot", slot_index), |ui| {
                Frame::new()
                    .fill(palette.pill_bg)
                    .stroke(Stroke::new(1.0, palette.pill_border))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            let mut chosen = dest_id;
                            ComboBox::from_id_salt("dest-picker")
                                .width(260.0)
                                .truncate()
                                .selected_text(destination_choice_label(current))
                                .show_ui(ui, |ui| {
                                    for candidate in destinations.iter() {
                                        let used_elsewhere = source
                                            .destination_ids
                                            .iter()
                                            .enumerate()
                                            .any(|(i, id)| i != slot_index && *id == candidate.id);
                                        if !used_elsewhere {
                                            ui.selectable_value(
                                                &mut chosen,
                                                candidate.id,
                                                destination_choice_label(candidate),
                                            );
                                        }
                                    }
                                });
                            if chosen != dest_id {
                                replace_slot = Some((slot_index, chosen));
                            }
                            if compact_button(ui, "Redigera", true, None, palette).clicked() {
                                outcome.open_destinations = true;
                            }
                            if compact_button(
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
                        ui.label(
                            RichText::new(destination_summary(current))
                                .size(12.5)
                                .color(palette.text_secondary),
                        );
                    });
            });
        }
        if let Some((slot_index, new_id)) = replace_slot {
            source.destination_ids[slot_index] = new_id;
            outcome.changed = true;
        }
        if let Some(slot_index) = remove_slot {
            source.destination_ids.remove(slot_index);
            outcome.changed = true;
        }

        let available: Vec<(u64, String)> = destinations
            .iter()
            .filter(|dest| !source.destination_ids.contains(&dest.id))
            .map(|dest| (dest.id, destination_choice_label(dest)))
            .collect();
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let mut add_existing = None;
            ui.add_enabled_ui(!available.is_empty(), |ui| {
                ComboBox::from_id_salt("add-existing-dest")
                    .width(230.0)
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
                .stroke(Stroke::new(1.0, palette.pill_border))
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
                        destination_settings(ui, draft, palette);
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if text_button(ui, "Skapa och välj", true, ButtonVariant::Neutral, palette)
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

    ui.add_space(10.0);

    // Section 3: Lokal inspelning
    card_frame(palette, palette.card_border).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(strong_text("Lokal inspelning", 15.5, palette.text_primary));
            crate::guide::tip(ui, "Välj var filen ska sparas. Mappknappen låter dig bläddra.");
        });
        ui.add_space(4.0);
        // Leave room for the label and both folder controls in a narrow window.
        let folder_field_width = (ui.available_width() - 310.0).clamp(174.0, 248.0);
        egui::Grid::new("recording-fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                let folder_label = ui.label("Mapp").id;
                let error_id = ui.make_persistent_id("open-recording-folder-error");
                let folder = &mut source.recording.folder;
                ui.horizontal(|ui| {
                    let input = ui
                        .add(
                            egui::TextEdit::singleline(folder)
                                .margin(Margin::symmetric(6, 4))
                                .desired_width(folder_field_width),
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
                            if let Some(error) = error {
                                data.insert_temp(error_id, error);
                            } else {
                                data.remove::<String>(error_id);
                            }
                        });
                    }
                });
                ui.end_row();
                if folder.trim().is_empty() {
                    field_hint(ui, "Välj en mapp för att kunna spela in.", palette);
                } else if let Some(error) = ui.data(|data| data.get_temp::<String>(error_id)) {
                    field_hint(ui, &error, palette);
                }
                ui.label("Filnamnsformat");
                let current_preset = FILENAME_FORMATS
                    .iter()
                    .find(|(_, template)| *template == source.recording.filename_format)
                    .map_or("Anpassat", |(label, _)| *label);
                ComboBox::from_id_salt("filename-format")
                    .width(260.0)
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
                ui.end_row();
                let template_label = ui.label("Filnamnsmall").id;
                outcome.changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut source.recording.filename_format)
                            .margin(Margin::symmetric(6, 4))
                            .desired_width(290.0),
                    )
                    .labelled_by(template_label)
                    .on_hover_text(
                        "Platshållare: {namn}, {datum}, {tid}, {år}, {månad}, {dag}, {timme}, {minut}, {sekund}, {unix}",
                    )
                    .changed();
                ui.end_row();
                ui.label("Exempel");
                ui.label(
                    RichText::new(crate::output::preview_recording_filename(
                        &source.recording.filename_format,
                        &source.name,
                        &source.recording.format,
                    ))
                    .color(palette.text_secondary),
                );
                ui.end_row();
                ui.label("Filformat");
                let was_lossy = bitrate_range(&source.recording.format).is_some();
                let format_changed = choice(
                    ui,
                    "format",
                    &mut source.recording.format,
                    &["WAV", "FLAC", "Opus", "MP3"],
                );
                outcome.changed |= format_changed;
                ui.end_row();
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
                    ComboBox::from_id_salt("bit-depth")
                        .selected_text(format!("{} bitar", source.recording.quality))
                        .show_ui(ui, |ui| {
                            for value in [16, 24] {
                                outcome.changed |= ui
                                    .selectable_value(
                                        &mut source.recording.quality,
                                        value,
                                        format!("{value} bitar"),
                                    )
                                    .changed();
                            }
                        });
                }
                ui.end_row();
            });
    });

    outcome
}

fn destination_settings(
    ui: &mut EguiUi,
    destination: &mut Destination,
    palette: Palette,
) -> bool {
    let mut changed = false;
    egui::Grid::new("destination-fields")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            changed |= field(ui, "Namn", &mut destination.name).changed();
            changed |= validated_field(ui, "Server-URL", &mut destination.server, server_hint, palette);
            changed |= validated_field(
                ui,
                "Monteringspunkt",
                &mut destination.mount,
                mount_hint,
                palette,
            );
            changed |= field(ui, "Användarnamn", &mut destination.user).changed();
            let password_label = ui.label("Lösenord").id;
            let show_id = ui.make_persistent_id("show-password");
            let mut show_password =
                ui.data(|data| data.get_temp::<bool>(show_id).unwrap_or(false));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut destination.password)
                            .password(!show_password)
                            .margin(Margin::symmetric(6, 4))
                            .desired_width(236.0),
                    )
                    .labelled_by(password_label)
                    .changed();
                let toggle_label = if show_password { "Dölj" } else { "Visa" };
                if compact_button(ui, toggle_label, true, None, palette).clicked() {
                    show_password = !show_password;
                    ui.data_mut(|data| data.insert_temp(show_id, show_password));
                }
            });
            ui.end_row();
            ui.label("Kodek");
            let codec_changed = choice(ui, "codec", &mut destination.codec, &["Opus", "MP3"]);
            changed |= codec_changed;
            ui.end_row();
            ui.label("Bithastighet");
            let range = bitrate_range(&destination.codec).unwrap_or(16..=510);
            if codec_changed {
                destination.bitrate = destination.bitrate.clamp(*range.start(), *range.end());
            }
            changed |= ui
                .add(
                    egui::DragValue::new(&mut destination.bitrate)
                        .range(range)
                        .suffix(" kbit/s"),
                )
                .changed();
            ui.end_row();
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
        .map_err(|error| format!("Kunde inte läsa mappsökvägen. {error}"))?;
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
        .map_err(|error| format!("Kunde inte öppna mappen. {error}"))
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
    if rest.split(['/', '?', '#']).next().is_some_and(|authority| authority.ends_with(':')) {
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

fn field_hint(ui: &mut EguiUi, hint: &str, palette: Palette) {
    ui.label("");
    ui.scope(|ui| {
        ui.set_max_width(290.0);
        ui.add(egui::Label::new(RichText::new(hint).color(palette.danger)).wrap());
    });
    ui.end_row();
}

fn validated_field(
    ui: &mut EguiUi,
    label: &str,
    value: &mut String,
    validate: fn(&str) -> Option<&'static str>,
    palette: Palette,
) -> bool {
    let input = field(ui, label, value);
    if let Some(hint) = validate(value) {
        mark_invalid(ui, &input, palette);
        field_hint(ui, hint, palette);
    }
    input.changed()
}

fn field(ui: &mut EguiUi, label: &str, value: &mut String) -> egui::Response {
    let id = ui.label(label).id;
    let input = ui
        .add(
            egui::TextEdit::singleline(value)
                .margin(Margin::symmetric(6, 4))
                .desired_width(290.0),
        )
        .labelled_by(id);
    ui.end_row();
    input
}

fn choice(ui: &mut EguiUi, id: &str, current: &mut String, values: &[&str]) -> bool {
    let mut changed = false;
    ComboBox::from_id_salt(id)
        .selected_text(current.as_str())
        .show_ui(ui, |ui| {
            for &value in values {
                changed |= ui
                    .selectable_value(current, value.to_owned(), value)
                    .changed();
            }
        });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for width in [820.0, 560.0, 560.0] {
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
                                &devices,
                                Palette::for_theme(false),
                            );
                            assert!(
                                ui.min_rect().right() <= right + 1.0,
                                "settings extend to {} beyond {right}",
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
                            let rec_count =
                                header_count(ui, 1, 3, palette.record_accent, palette);
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
                            let str_count =
                                header_count(ui, 2, 3, palette.stream_accent, palette);
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
                            rects = [
                                str_start.rect,
                                str_stop.rect,
                                rec_start.rect,
                                rec_stop.rect,
                            ];
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
        assert!(build_meter_gradient_mesh(rect, 0.0, palette).vertices.is_empty());

        let quiet_mesh = build_meter_gradient_mesh(rect, 0.5, palette);
        assert_eq!(quiet_mesh.vertices.len(), 4);
        assert!(quiet_mesh.vertices.iter().all(|v| v.color == palette.meter_ok));

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
        assert_eq!(mid_mesh.vertices[mid_mesh.vertices.len() - 2].color, palette.meter_warn);

        let full_mesh = build_meter_gradient_mesh(rect, 1.0, palette);
        assert_eq!(full_mesh.vertices.len(), 20);
        assert_eq!(full_mesh.vertices.first().unwrap().color, palette.meter_ok);
        assert_eq!(full_mesh.vertices[full_mesh.vertices.len() - 2].color, palette.meter_peak);
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
                            meter(ui, "Vänster", MeterLevel { signal: 0.5, ..Default::default() }, false, palette);
                            meter(ui, "Höger", MeterLevel { signal: 0.9, ..Default::default() }, true, palette);
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
                    destination_settings(ui, &mut dest_a, palette);
                    id_a = ui.make_persistent_id("show-password");
                });
                ui.push_id(dest_b.id, |ui| {
                    destination_settings(ui, &mut dest_b, palette);
                    id_b = ui.make_persistent_id("show-password");
                });
            });
        });
        assert_ne!(id_a, id_b);
        assert!(!ctx.data(|d| d.get_temp::<bool>(id_a).unwrap_or(false)));
        assert!(!ctx.data(|d| d.get_temp::<bool>(id_b).unwrap_or(false)));

        ctx.data_mut(|d| d.insert_temp(id_a, true));
        assert!(ctx.data(|d| d.get_temp::<bool>(id_a).unwrap_or(false)));
        assert!(!ctx.data(|d| d.get_temp::<bool>(id_b).unwrap_or(false)));
    }
}
