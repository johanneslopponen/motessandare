use crate::{Destination, Engine, Settings, Source, settings_load, settings_save};
use eframe::egui::{
    self, Align, Color32, ColorImage, ComboBox, Context, CornerRadius, FontData, FontDefinitions,
    FontFamily, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense, Shadow,
    Shape, Stroke, TextureHandle, TextureOptions, Ui as EguiUi, Vec2,
};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

const STRONG_FONT: &str = "strong";

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
    Broadcast,
    BroadcastStop,
    Record,
    RecordStop,
    Stop,
    Settings,
    Plus,
    Save,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Overview,
    SourceSettings,
    Destinations,
}

pub struct Ui {
    engine: Engine,
    settings: Settings,
    selected_source: Option<u64>,
    view: View,
    dirty: bool,
    notice: Option<String>,
    remove_source: bool,
    remove_destination: Option<u64>,
    new_destination_draft: Option<Destination>,
    app_icon: Option<TextureHandle>,
    close_blocked: bool,
    load_blocked: bool,
}

impl Ui {
    pub fn new(ctx: &Context) -> Self {
        configure_fonts(ctx);
        ctx.set_visuals_of(egui::Theme::Dark, build_visuals(true));
        ctx.set_visuals_of(egui::Theme::Light, build_visuals(false));
        ctx.style_mut(|style| {
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
            dirty: false,
            notice,
            remove_source: false,
            remove_destination: None,
            new_destination_draft: None,
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
        self.selected_source = Some(id);
        self.navigate(View::SourceSettings);
        self.dirty = true;
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
        self.dirty = true;
    }

    fn destination_streaming(&self, destination_id: u64) -> bool {
        self.settings.sources.iter().any(|source| {
            source.destination_ids.contains(&destination_id) && self.engine.streaming(source.id)
        })
    }

    fn save(&mut self) {
        match settings_save(&self.settings) {
            Ok(()) => {
                self.notice = None;
                self.dirty = false;
                self.close_blocked = false;
            }
            Err(err) => self.notice = Some(format!("Inställningarna kunde inte sparas. {err}")),
        }
    }

    fn header(&mut self, ui: &mut EguiUi, palette: Palette) {
        let can_start_streams = self.settings.sources.iter().any(|source| {
            !source.device.is_empty()
                && !source
                    .resolve_destinations(&self.settings.destinations)
                    .is_empty()
                && !self.engine.streaming(source.id)
        });
        let can_stop_streams = self
            .settings
            .sources
            .iter()
            .any(|source| self.engine.streaming(source.id));
        let can_start_recordings = self.settings.sources.iter().any(|source| {
            !source.device.is_empty()
                && !source.recording.folder.trim().is_empty()
                && !self.engine.recording(source.id)
        });
        let can_stop_recordings = self
            .settings
            .sources
            .iter()
            .any(|source| self.engine.recording(source.id));

        ui.horizontal(|ui| {
            ui.set_min_height(36.0);
            ui.spacing_mut().item_spacing.x = 10.0;
            if let Some(icon) = &self.app_icon {
                ui.add(
                    egui::Image::new((icon.id(), Vec2::splat(26.0)))
                        .corner_radius(CornerRadius::same(6)),
                );
            }
            ui.label(strong_text("Mötessändare", 18.0, palette.text_primary));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.set_height(36.0);
                ui.spacing_mut().item_spacing.x = 8.0;

                // Recording bulk controls pill (far right).
                let (record_start, record_stop) =
                    header_pill(ui, "record-pill", palette, |ui| {
                    let start = icon_button(
                        ui,
                        Icon::Record,
                        can_start_recordings,
                        palette.record_accent,
                        false,
                        palette,
                        "Starta alla inspelningar",
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
                    for source in &self.settings.sources {
                        if self.engine.recording(source.id) {
                            self.engine.set_record(source, false);
                        }
                    }
                }

                // Streaming bulk controls pill (left of recording pill).
                let (stream_start, stream_stop) =
                    header_pill(ui, "stream-pill", palette, |ui| {
                    let start = icon_button(
                        ui,
                        Icon::Broadcast,
                        can_start_streams,
                        palette.stream_accent,
                        false,
                        palette,
                        "Starta alla sändningar",
                    );
                    let stop = icon_button(
                        ui,
                        Icon::BroadcastStop,
                        can_stop_streams,
                        palette.record_accent,
                        false,
                        palette,
                        "Stoppa alla sändningar",
                    );
                    (start, stop)
                });
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
                if stream_stop.clicked() {
                    for source in &self.settings.sources {
                        if self.engine.streaming(source.id) {
                            self.engine
                                .set_stream(source, &self.settings.destinations, false);
                        }
                    }
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
                    self.dirty = false;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let palette = Palette::for_theme(ctx.style().visuals.dark_mode);

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

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(palette.canvas_bg)
                    .inner_margin(Margin::symmetric(16, 12)),
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

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_height(30.0);
                        ui.spacing_mut().item_spacing.x = 8.0;

                        let save_variant = if self.dirty {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Neutral
                        };
                        let save_icon_color = if self.dirty {
                            palette.tab_active_fg
                        } else {
                            palette.text_disabled
                        };
                        if action_button(
                            ui,
                            Icon::Save,
                            "Spara inställningar",
                            self.dirty,
                            save_icon_color,
                            save_variant,
                            palette,
                        )
                        .clicked()
                        {
                            self.save();
                        }

                        if self.view != View::Destinations
                            && action_button(
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
                            self.add_source();
                        }

                        if self.dirty {
                            ui.label(
                                RichText::new("Osparade ändringar")
                                    .size(12.5)
                                    .color(palette.warning),
                            );
                        }
                    });
                });

                ui.add_space(10.0);

                match self.view {
                    View::Overview => {
                        if self.settings.sources.is_empty() {
                            empty_sources_view(ui, palette, || self.add_source());
                            return;
                        }
                        let mut open_settings_id = None;
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
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
                                            palette,
                                        );
                                        if clicked_settings {
                                            open_settings_id = Some(source.id);
                                        }
                                        ui.add_space(10.0);
                                    });
                                }
                            });
                        if let Some(id) = open_settings_id {
                            self.selected_source = Some(id);
                            self.navigate(View::SourceSettings);
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
                        let mut go_overview = false;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if compact_button(ui, "← Översikt", true, None, palette).clicked() {
                                go_overview = true;
                            }
                            ui.label(
                                RichText::new(format!(
                                    " / {}",
                                    display_name(&self.settings.sources[index])
                                ))
                                .size(13.5)
                                .color(palette.text_secondary),
                            );
                        });
                        if go_overview {
                            self.navigate(View::Overview);
                            return;
                        }
                        ui.add_space(6.0);
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
                        } = &mut self.settings;
                        let source = &mut sources[index];
                        ScrollArea::vertical()
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
                            self.dirty = true;
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
                                        "Spara sändningsdestinationer globalt så kan de väljas av en eller flera ljudkällor.",
                                    )
                                    .color(palette.text_secondary),
                                );
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
                                                changed |= destination_settings(ui, dest);
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
                            self.dirty = true;
                            self.engine.update(&self.settings.sources);
                        }
                    }
                }
            });

        if self.dirty && ctx.input(|input| input.viewport().close_requested()) {
            self.save();
            if self.dirty {
                self.close_blocked = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        ctx.request_repaint_after(Duration::from_millis(65));
    }
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
        Icon::Save => {
            // Minimalist save / diskette outline
            let outer = Rect::from_center_size(c, Vec2::splat(r * 1.48));
            painter.rect_stroke(
                outer,
                CornerRadius::same(2),
                Stroke::new(1.4, color),
                egui::StrokeKind::Inside,
            );
            let top = Rect::from_min_max(
                Pos2::new(outer.left() + r * 0.32, outer.top()),
                Pos2::new(outer.right() - r * 0.32, outer.top() + r * 0.56),
            );
            painter.rect_stroke(
                top,
                CornerRadius::same(1),
                Stroke::new(1.2, color),
                egui::StrokeKind::Inside,
            );
            let bottom = Rect::from_min_max(
                Pos2::new(outer.left() + r * 0.32, outer.bottom() - r * 0.54),
                Pos2::new(outer.right() - r * 0.32, outer.bottom()),
            );
            painter.rect_filled(bottom, CornerRadius::same(1), color);
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
}

fn paint_focus_ring(painter: &egui::Painter, rect: Rect, corner_radius: u8, palette: Palette) {
    painter.rect_stroke(
        rect.expand(1.0),
        CornerRadius::same(corner_radius.saturating_add(1)),
        Stroke::new(2.0, palette.stream_accent),
        egui::StrokeKind::Outside,
    );
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
    let is_enabled = spec.enabled && ui.is_enabled();
    let font_id = FontId::proportional(spec.font_size);
    let text_galley = ui
        .painter()
        .layout_no_wrap(spec.label.to_owned(), font_id, Color32::PLACEHOLDER);
    let icon_size = 15.0;
    let gap = 7.0;
    let width = if spec.icon.is_some() {
        spec.pad_x + icon_size + gap + text_galley.size().x + spec.pad_x
    } else {
        spec.pad_x + text_galley.size().x + spec.pad_x
    };

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
    let (bg, border, text_color, fg_icon) = if !is_enabled {
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

    if let Some((icon, _)) = spec.icon {
        let icon_center = Pos2::new(rect.left() + spec.pad_x + icon_size * 0.5, rect.center().y);
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(icon_size));
        paint_icon(painter, icon_rect, icon, fg_icon);

        let text_pos = Pos2::new(
            rect.left() + spec.pad_x + icon_size + gap,
            rect.center().y - text_galley.size().y * 0.5,
        );
        painter.galley(text_pos, text_galley, text_color);
    } else {
        let text_pos = Pos2::new(
            rect.center().x - text_galley.size().x * 0.5,
            rect.center().y - text_galley.size().y * 0.5,
        );
        painter.galley(text_pos, text_galley, text_color);
    }

    response
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
    text: impl Into<String>,
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
            ui.label(RichText::new(text).size(12.0).color(fg));
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
            RichText::new("Välj en ljudingång och kanaler för sändning eller inspelning.")
                .color(palette.text_secondary),
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

#[allow(clippy::too_many_arguments)]
fn source_card(
    ui: &mut EguiUi,
    engine: &Engine,
    source: &Source,
    destinations: &[Destination],
    streaming: bool,
    recording: bool,
    show_settings_button: bool,
    palette: Palette,
) -> bool {
    let mut open_settings = false;
    let border_color = if recording {
        palette.record_active_border
    } else if streaming {
        palette.stream_active_border
    } else {
        palette.card_border
    };

    card_frame(palette, border_color).show(ui, |ui| {
        ui.set_width(ui.available_width());

        // Card header: title + active badges on left, settings button on right
        ui.horizontal(|ui| {
            ui.label(strong_text(display_name(source), 16.5, palette.text_primary));
            if streaming {
                status_pill(
                    ui,
                    "Sänder",
                    palette.stream_pill_bg,
                    palette.stream_active_border,
                    palette.stream_accent,
                );
            }
            if recording {
                status_pill(
                    ui,
                    "Spelar in",
                    palette.record_pill_bg,
                    palette.record_active_border,
                    palette.record_accent,
                );
            }

            if show_settings_button {
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

        // Subtle divider inside the card
        ui.add_space(4.0);
        let (sep_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter()
            .rect_filled(sep_rect, CornerRadius::ZERO, palette.card_divider);
        ui.add_space(6.0);

        let present = engine.present(source.id);
        let status = engine.status(source.id);
        if !present {
            let message = if source.device.is_empty() {
                "Välj en ljudingång i inställningarna."
            } else {
                "Väntar på ljudenheten. Startade utgångar får tyst signal."
            };
            ui.label(RichText::new(message).size(12.5).color(palette.warning));
        }
        if !status.is_empty() {
            ui.label(RichText::new(&status).size(12.5).color(palette.text_secondary));
        }

        let (left, right) = if present {
            engine.level(source.id)
        } else {
            (0.0, 0.0)
        };
        ui.push_id(source.id, |ui| {
            if source.right.is_some() {
                meter(ui, "Vänster", left, palette);
                meter(ui, "Höger", right, palette);
            } else {
                meter(ui, "Mono", left, palette);
            }
        });

        ui.add_space(4.0);

        let count = source.resolve_destinations(destinations).len();
        ui.horizontal_wrapped(|ui| {
            let stream_enabled = streaming || (!source.device.is_empty() && count > 0);
            let (stream_icon, stream_label, stream_variant) = if streaming {
                (Icon::Stop, "Stoppa sändning", ButtonVariant::StreamActive)
            } else {
                (Icon::Broadcast, "Starta sändning", ButtonVariant::Neutral)
            };
            if action_button(
                ui,
                stream_icon,
                stream_label,
                stream_enabled,
                palette.stream_accent,
                stream_variant,
                palette,
            )
            .clicked()
            {
                engine.set_stream(source, destinations, !streaming);
            }

            let record_enabled = recording
                || (!source.device.is_empty() && !source.recording.folder.trim().is_empty());
            let (record_icon, record_label, record_variant) = if recording {
                (Icon::Stop, "Stoppa inspelning", ButtonVariant::RecordActive)
            } else {
                (Icon::Record, "Starta inspelning", ButtonVariant::Neutral)
            };
            if action_button(
                ui,
                record_icon,
                record_label,
                record_enabled,
                palette.record_accent,
                record_variant,
                palette,
            )
            .clicked()
            {
                engine.set_record(source, !recording);
            }

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
        });
    });

    open_settings
}

fn meter_fraction(signal: f32) -> f32 {
    if signal.is_finite() && signal > 0.0 {
        (1.0 + signal.log10() / 3.0).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn meter(ui: &mut EguiUi, label: &str, signal: f32, palette: Palette) {
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
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_width, 10.0), Sense::hover());
        let painter = ui.painter();
        let rounding = CornerRadius::same(5);
        painter.rect_filled(bar_rect, rounding, palette.meter_track);
        painter.rect_stroke(
            bar_rect,
            rounding,
            Stroke::new(1.0, palette.meter_border),
            egui::StrokeKind::Inside,
        );

        let frac = meter_fraction(signal);
        if frac > 0.002 {
            let fill_w = (bar_rect.width() * frac).max(6.0).min(bar_rect.width());
            let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
            let fill_color = if signal >= 0.95 {
                palette.meter_peak
            } else if signal >= 0.5 {
                palette.meter_warn
            } else {
                palette.meter_ok
            };
            painter.rect_filled(fill_rect, rounding, fill_color);
        }

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
        ui.label(strong_text("Ljudingång", 15.5, palette.text_primary));
        ui.add_space(4.0);
        egui::Grid::new("input-fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                outcome.changed |= field(ui, "Namn", &mut source.name, false);
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
                ComboBox::from_id_salt("device")
                    .width(260.0)
                    .selected_text(selected_name)
                    .show_ui(ui, |ui| {
                        if devices.is_empty() {
                            ui.label(
                                RichText::new("Inga ljudenheter tillgängliga")
                                    .color(palette.text_secondary),
                            );
                        }
                        for (id, name, _) in devices {
                            ui.selectable_value(&mut device, id.clone(), name);
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
        ui.label(strong_text(
            "Sändningsdestinationer",
            15.5,
            palette.text_primary,
        ));
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
                        destination_settings(ui, draft);
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
        ui.label(strong_text("Lokal inspelning", 15.5, palette.text_primary));
        ui.add_space(4.0);
        egui::Grid::new("recording-fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                outcome.changed |= field(ui, "Mapp", &mut source.recording.folder, false);
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
                            .desired_width(260.0),
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

fn destination_settings(ui: &mut EguiUi, destination: &mut Destination) -> bool {
    let mut changed = false;
    egui::Grid::new("destination-fields")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            changed |= field(ui, "Namn", &mut destination.name, false);
            changed |= field(ui, "Server-URL", &mut destination.server, false);
            changed |= field(ui, "Monteringspunkt", &mut destination.mount, false);
            changed |= field(ui, "Användarnamn", &mut destination.user, false);
            changed |= field(ui, "Lösenord", &mut destination.password, true);
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

fn field(ui: &mut EguiUi, label: &str, value: &mut String, secret: bool) -> bool {
    let id = ui.label(label).id;
    let changed = ui
        .add(
            egui::TextEdit::singleline(value)
                .password(secret)
                .desired_width(260.0),
        )
        .labelled_by(id)
        .changed();
    ui.end_row();
    changed
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
                            ui.spacing_mut().item_spacing.x = 8.0;
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
                                        palette.record_accent,
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
        assert!(
            str_start.right() < str_stop.left(),
            "stream start ({str_start:?}) should be left of stream stop ({str_stop:?})"
        );
        assert!(
            str_stop.right() < rec_start.left(),
            "stream pill ({str_stop:?}) should be left of record pill ({rec_start:?})"
        );
        assert!(
            rec_start.right() < rec_stop.left(),
            "record start ({rec_start:?}) should be left of record stop ({rec_stop:?})"
        );
        for other in [str_stop, rec_start, rec_stop] {
            assert!(
                (str_start.center().y - other.center().y).abs() < 0.5,
                "buttons should share vertical center: {str_start:?} vs {other:?}"
            );
        }
        assert!(
            str_start.left() > 400.0,
            "bulk pills should be right-aligned, not stretched: {str_start:?}"
        );
    }
}
