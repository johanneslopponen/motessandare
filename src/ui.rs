use crate::{Destination, Engine, Source, settings_load, settings_save};
use eframe::egui::{
    self, Align, Color32, ComboBox, Context, Frame, Layout, ProgressBar, RichText, ScrollArea,
    Ui as EguiUi, Vec2,
};
use std::time::Duration;

const ACCENT: Color32 = Color32::from_rgb(100, 217, 191);
const WARNING: Color32 = Color32::from_rgb(246, 192, 113);
const FILENAME_FORMATS: &[(&str, &str)] = &[
    ("Namn – Datum – Tid", "{namn}-{datum}-{tid}"),
    ("Datum – Tid – Namn", "{datum}-{tid}-{namn}"),
    ("Datum – Namn", "{datum}-{namn}"),
    ("Namn – Datum", "{namn}-{datum}"),
    ("Namn – Unix-tid", "{namn}-{unix}"),
];

pub struct Ui {
    engine: Engine,
    sources: Vec<Source>,
    selected: Option<u64>,
    configure: bool,
    dirty: bool,
    notice: Option<String>,
    remove_source: bool,
    styled: bool,
    close_blocked: bool,
    load_blocked: bool,
}

impl Ui {
    pub fn new() -> Self {
        let (sources, notice) = match settings_load() {
            Ok(sources) => (sources, None),
            Err(err) => (
                Vec::new(),
                Some(format!("Inställningarna kunde inte läsas. {err}")),
            ),
        };
        let load_blocked = notice.is_some();
        let engine = Engine::new();
        engine.update(&sources);
        Self {
            engine,
            selected: sources.first().map(|source| source.id),
            sources,
            configure: false,
            dirty: false,
            notice,
            remove_source: false,
            styled: false,
            close_blocked: false,
            load_blocked,
        }
    }

    fn add_source(&mut self) {
        if self.load_blocked {
            return;
        }
        let mut id = 1;
        while self.sources.iter().any(|source| source.id == id) {
            id += 1;
        }
        self.sources.push(Source {
            id,
            name: format!("Ljudkälla {id}"),
            destinations: Vec::new(),
            ..Source::default()
        });
        self.selected = Some(id);
        self.configure = true;
        self.remove_source = false;
        self.dirty = true;
        self.engine.update(&self.sources);
    }

    fn save(&mut self) {
        match settings_save(&self.sources) {
            Ok(()) => {
                self.notice = None;
                self.dirty = false;
                self.close_blocked = false;
            }
            Err(err) => self.notice = Some(format!("Inställningarna kunde inte sparas. {err}")),
        }
    }

    fn header(&mut self, ui: &mut EguiUi) {
        ui.horizontal_wrapped(|ui| {
            ui.heading(RichText::new("Mötessändare").size(25.0));
            ui.separator();
            let can_start_streams = self.sources.iter().any(|source| {
                !source.device.is_empty()
                    && !source.destinations.is_empty()
                    && !self.engine.streaming(source.id)
            });
            let can_stop_streams = self
                .sources
                .iter()
                .any(|source| self.engine.streaming(source.id));
            if ui
                .add_enabled(
                    can_start_streams,
                    egui::Button::new("Starta alla sändningar"),
                )
                .clicked()
            {
                for source in &self.sources {
                    if !source.device.is_empty()
                        && !source.destinations.is_empty()
                        && !self.engine.streaming(source.id)
                    {
                        self.engine.set_stream(source, true);
                    }
                }
            }
            if ui
                .add_enabled(can_stop_streams, egui::Button::new("Stoppa alla sändningar"))
                .clicked()
            {
                for source in &self.sources {
                    if self.engine.streaming(source.id) {
                        self.engine.set_stream(source, false);
                    }
                }
            }
            let can_start_recordings = self.sources.iter().any(|source| {
                !source.device.is_empty()
                    && !source.recording.folder.trim().is_empty()
                    && !self.engine.recording(source.id)
            });
            let can_stop_recordings = self
                .sources
                .iter()
                .any(|source| self.engine.recording(source.id));
            if ui
                .add_enabled(
                    can_start_recordings,
                    egui::Button::new("Starta alla inspelningar"),
                )
                .clicked()
            {
                for source in &self.sources {
                    if !source.device.is_empty()
                        && !source.recording.folder.trim().is_empty()
                        && !self.engine.recording(source.id)
                    {
                        self.engine.set_record(source, true);
                    }
                }
            }
            if ui
                .add_enabled(
                    can_stop_recordings,
                    egui::Button::new("Stoppa alla inspelningar"),
                )
                .clicked()
            {
                for source in &self.sources {
                    if self.engine.recording(source.id) {
                        self.engine.set_record(source, false);
                    }
                }
            }
        });
        if let Some(message) = &self.notice {
            ui.label(RichText::new(message).color(WARNING));
            if self.load_blocked && ui.button("Läs in igen").clicked() {
                match settings_load() {
                    Ok(sources) => {
                        self.selected = sources.first().map(|source| source.id);
                        self.sources = sources;
                        self.engine.update(&self.sources);
                        self.load_blocked = false;
                        self.notice = None;
                    }
                    Err(err) => {
                        self.notice = Some(format!("Inställningarna kunde inte läsas. {err}"));
                    }
                }
            }
            if self.close_blocked && ui.button("Stäng utan att spara").clicked() {
                self.dirty = false;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if !self.styled {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(23, 28, 36);
            visuals.window_fill = visuals.panel_fill;
            visuals.selection.bg_fill = Color32::from_rgb(39, 88, 86);
            ctx.set_visuals(visuals);
            ctx.style_mut(|style| {
                style.spacing.item_spacing = Vec2::new(10.0, 8.0);
                style.spacing.button_padding = Vec2::new(10.0, 6.0);
            });
            self.styled = true;
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            self.header(ui);
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.load_blocked {
                return;
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_value(&mut self.configure, false, "Översikt")
                    .clicked()
                {
                    self.remove_source = false;
                }
                if ui.button("+ Lägg till ljudkälla").clicked() {
                    self.add_source();
                }
                if ui
                    .add_enabled(self.dirty, egui::Button::new("Spara inställningar"))
                    .clicked()
                {
                    self.save();
                }
                if self.dirty {
                    ui.label(RichText::new("Osparade ändringar").color(WARNING));
                }
            });
            ui.separator();
            if self.sources.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading("Lägg till en ljudkälla");
                    ui.label("Välj en ljudingång och kanaler för sändning eller inspelning.");
                    if ui.button("Lägg till ljudkälla").clicked() {
                        self.add_source();
                    }
                });
                return;
            }
            if !self.configure {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for source in &self.sources {
                            ui.push_id(source.id, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.heading(display_name(source));
                                    if ui.small_button("Inställningar").clicked() {
                                        self.selected = Some(source.id);
                                        self.configure = true;
                                        self.remove_source = false;
                                    }
                                });
                                source_controls(
                                    ui,
                                    &self.engine,
                                    source,
                                    self.engine.streaming(source.id),
                                    self.engine.recording(source.id),
                                );
                                ui.add_space(8.0);
                            });
                        }
                    });
                return;
            }

            let Some(index) = self
                .sources
                .iter()
                .position(|source| Some(source.id) == self.selected)
            else {
                self.configure = false;
                return;
            };

            let source = &mut self.sources[index];
            let streaming = self.engine.streaming(source.id);
            let recording = self.engine.recording(source.id);
            let active = streaming || recording;
            let mut removed = false;
            let mut changed = false;
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading(display_name(source));
                    source_controls(ui, &self.engine, source, streaming, recording);
                    if active {
                        ui.label(
                            RichText::new(
                                "Stoppa sändning och inspelning för att ändra inställningarna.",
                            )
                            .color(WARNING),
                        );
                    }
                    let devices = self.engine.devices();
                    ui.add_enabled_ui(!active, |ui| {
                        changed |= ui
                            .push_id(source.id, |ui| source_settings(ui, source, &devices))
                            .inner;
                        ui.separator();
                        if self.remove_source {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Ta bort den här ljudkällan?");
                                removed = ui.button("Ta bort").clicked();
                                if ui.button("Avbryt").clicked() {
                                    self.remove_source = false;
                                }
                            });
                        } else if ui.button("Ta bort ljudkälla").clicked() {
                            self.remove_source = true;
                        }
                    });
                });
            if removed {
                self.sources.remove(index);
                self.selected = self
                    .sources
                    .get(index)
                    .or_else(|| self.sources.last())
                    .map(|source| source.id);
                self.configure = false;
                self.remove_source = false;
            }
            if changed || removed {
                self.dirty = true;
                self.engine.update(&self.sources);
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

fn display_name(source: &Source) -> &str {
    if source.name.trim().is_empty() {
        "Namnlös ljudkälla"
    } else {
        &source.name
    }
}

fn source_controls(
    ui: &mut EguiUi,
    engine: &Engine,
    source: &Source,
    streaming: bool,
    recording: bool,
) {
    Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let present = engine.present(source.id);
        let status = engine.status(source.id);
        if !present {
            let message = if source.device.is_empty() {
                "Välj en ljudingång i inställningarna."
            } else {
                "Väntar på ljudenheten. Startade utgångar får tyst signal."
            };
            ui.label(RichText::new(message).color(WARNING));
        }
        if !status.is_empty() {
            ui.label(&status);
        }
        let (left, right) = if present {
            engine.level(source.id)
        } else {
            (0.0, 0.0)
        };
        ui.push_id(source.id, |ui| {
            if source.right.is_some() {
                meter(ui, "Vänster", left);
                meter(ui, "Höger", right);
            } else {
                meter(ui, "Mono", left);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    streaming || (!source.device.is_empty() && !source.destinations.is_empty()),
                    egui::Button::new(if streaming {
                        "Stoppa sändning"
                    } else {
                        "Starta sändning"
                    }),
                )
                .clicked()
            {
                engine.set_stream(source, !streaming);
            }
            if ui
                .add_enabled(
                    recording
                        || (!source.device.is_empty()
                            && !source.recording.folder.trim().is_empty()),
                    egui::Button::new(if recording {
                        "Stoppa inspelning"
                    } else {
                        "Starta inspelning"
                    }),
                )
                .clicked()
            {
                engine.set_record(source, !recording);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let count = source.destinations.len();
                ui.weak(if count == 1 {
                    "1 destination".to_owned()
                } else {
                    format!("{count} destinationer")
                });
            });
        });
    });
}

fn meter_fraction(signal: f32) -> f32 {
    if signal.is_finite() && signal > 0.0 {
        (1.0 + signal.log10() / 3.0).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn meter(ui: &mut EguiUi, label: &str, signal: f32) {
    ui.horizontal(|ui| {
        ui.add_sized([48.0, 20.0], egui::Label::new(label));
        let text = if signal.is_finite() && signal >= 0.001 {
            format!("{:.1} dBFS", 20.0 * signal.log10())
        } else {
            "−∞ dBFS".to_owned()
        };
        let color = if signal >= 0.99 {
            Color32::LIGHT_RED
        } else {
            ACCENT
        };
        ui.add(
            ProgressBar::new(meter_fraction(signal))
                .desired_width((ui.available_width() - 86.0).max(32.0))
                .desired_height(11.0)
                .fill(color),
        );
        ui.weak(text);
    });
}

fn source_settings(
    ui: &mut EguiUi,
    source: &mut Source,
    devices: &[(String, String, usize)],
) -> bool {
    let mut changed = false;
    ui.heading("Ljudingång");
    egui::Grid::new("input-fields")
        .num_columns(2)
        .show(ui, |ui| {
            changed |= field(ui, "Namn", &mut source.name, false);
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
                        ui.weak("Inga ljudenheter tillgängliga");
                    }
                    for (id, name, _) in devices {
                        ui.selectable_value(&mut device, id.clone(), name);
                    }
                });
            if device != source.device {
                if let Some((_, name, count)) = devices.iter().find(|(id, _, _)| id == &device) {
                    source.device = device;
                    source.device_name.clone_from(name);
                    source.left = source.left.min(count.saturating_sub(1));
                    source.right = source
                        .right
                        .filter(|right| *right < *count && *right != source.left);
                    changed = true;
                }
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
                        ui.selectable_value(&mut source.left, channel, (channel + 1).to_string());
                    }
                });
            if left != source.left {
                if old_right == Some(source.left) {
                    source.right = Some(left);
                }
                changed = true;
            }
            ui.end_row();
            ui.label("Höger kanal");
            ComboBox::from_id_salt("right")
                .selected_text(source.right.map_or_else(
                    || "Ingen (mono)".to_owned(),
                    |right| (right + 1).to_string(),
                ))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut source.right, None, "Ingen (mono)")
                        .changed();
                    for channel in (0..count).filter(|channel| *channel != source.left) {
                        changed |= ui
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

    ui.separator();
    ui.heading("Sändningsdestinationer");
    let mut remove = None;
    for (index, destination) in source.destinations.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Destination {}", index + 1));
                    if ui.small_button("Ta bort").clicked() {
                        remove = Some(index);
                    }
                });
                changed |= destination_settings(ui, destination);
            });
        });
    }
    if let Some(index) = remove {
        source.destinations.remove(index);
        changed = true;
    }
    if ui.button("+ Lägg till destination").clicked() {
        source.destinations.push(Destination::default());
        changed = true;
    }

    ui.separator();
    ui.heading("Lokal inspelning");
    egui::Grid::new("recording-fields")
        .num_columns(2)
        .show(ui, |ui| {
            changed |= field(ui, "Mapp", &mut source.recording.folder, false);
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
                            changed = true;
                        }
                    }
                });
            ui.end_row();
            let template_label = ui.label("Filnamnsmall").id;
            changed |= ui
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
            ui.weak(crate::output::preview_recording_filename(
                &source.recording.filename_format,
                &source.name,
                &source.recording.format,
            ));
            ui.end_row();
            ui.label("Filformat");
            let was_lossy = bitrate_range(&source.recording.format).is_some();
            let format_changed = choice(
                ui,
                "format",
                &mut source.recording.format,
                &["WAV", "FLAC", "Opus", "MP3"],
            );
            changed |= format_changed;
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
                changed |= ui
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
                            changed |= ui
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
    changed
}

fn destination_settings(ui: &mut EguiUi, destination: &mut Destination) -> bool {
    let mut changed = false;
    egui::Grid::new("destination-fields")
        .num_columns(2)
        .show(ui, |ui| {
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
