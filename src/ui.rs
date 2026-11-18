use crate::{Destination, Engine, Settings, Source, settings_load, settings_save};
use eframe::egui::{
    self, Align, Color32, ComboBox, Context, Frame, Layout, ProgressBar, RichText, ScrollArea,
    Ui as EguiUi, Vec2,
};
use std::time::Duration;

const ACCENT: Color32 = Color32::from_rgb(100, 217, 191);
const WARNING: Color32 = Color32::from_rgb(246, 192, 113);

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
    selected_destination: Option<u64>,
    view: View,
    dirty: bool,
    notice: Option<String>,
    remove_source: bool,
    remove_destination: bool,
    new_destination_draft: Option<Destination>,
    styled: bool,
    close_blocked: bool,
    load_blocked: bool,
}

impl Ui {
    pub fn new() -> Self {
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
        let selected_destination = settings.destinations.first().map(|dest| dest.id);
        Self {
            engine,
            settings,
            selected_source,
            selected_destination,
            view: View::Overview,
            dirty: false,
            notice,
            remove_source: false,
            remove_destination: false,
            new_destination_draft: None,
            styled: false,
            close_blocked: false,
            load_blocked,
        }
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
        self.view = View::SourceSettings;
        self.remove_source = false;
        self.new_destination_draft = None;
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
        self.selected_destination = Some(id);
        self.view = View::Destinations;
        self.remove_destination = false;
        self.new_destination_draft = None;
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

    fn header(&mut self, ui: &mut EguiUi) {
        ui.horizontal_wrapped(|ui| {
            ui.heading(RichText::new("Mötessändare").size(25.0));
            ui.separator();
            ui.menu_button("Sändningar", |ui| {
                let can_start = self.settings.sources.iter().any(|source| {
                    !source.device.is_empty()
                        && !source
                            .resolve_destinations(&self.settings.destinations)
                            .is_empty()
                        && !self.engine.streaming(source.id)
                });
                let can_stop = self
                    .settings
                    .sources
                    .iter()
                    .any(|source| self.engine.streaming(source.id));
                if ui
                    .add_enabled(can_start, egui::Button::new("Starta alla sändningar"))
                    .clicked()
                {
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
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_stop, egui::Button::new("Stoppa alla sändningar"))
                    .clicked()
                {
                    for source in &self.settings.sources {
                        if self.engine.streaming(source.id) {
                            self.engine
                                .set_stream(source, &self.settings.destinations, false);
                        }
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button("Inspelningar", |ui| {
                let can_start = self.settings.sources.iter().any(|source| {
                    !source.device.is_empty()
                        && !source.recording.folder.trim().is_empty()
                        && !self.engine.recording(source.id)
                });
                let can_stop = self
                    .settings
                    .sources
                    .iter()
                    .any(|source| self.engine.recording(source.id));
                if ui
                    .add_enabled(can_start, egui::Button::new("Starta alla inspelningar"))
                    .clicked()
                {
                    for source in &self.settings.sources {
                        if !source.device.is_empty()
                            && !source.recording.folder.trim().is_empty()
                            && !self.engine.recording(source.id)
                        {
                            self.engine.set_record(source, true);
                        }
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_stop, egui::Button::new("Stoppa alla inspelningar"))
                    .clicked()
                {
                    for source in &self.settings.sources {
                        if self.engine.recording(source.id) {
                            self.engine.set_record(source, false);
                        }
                    }
                    ui.close_menu();
                }
            });
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
        if let Some(message) = &self.notice {
            ui.label(RichText::new(message).color(WARNING));
            if self.load_blocked && ui.button("Läs in igen").clicked() {
                match settings_load() {
                    Ok(settings) => {
                        self.selected_source = settings.sources.first().map(|source| source.id);
                        self.selected_destination =
                            settings.destinations.first().map(|dest| dest.id);
                        self.settings = settings;
                        self.engine.update(&self.settings.sources);
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

        egui::SidePanel::left("sources")
            .default_width(224.0)
            .width_range(170.0..=360.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Ljudkällor");
                    if ui
                        .add_enabled(
                            !self.load_blocked,
                            egui::Button::new("+ Lägg till ljudkälla"),
                        )
                        .clicked()
                    {
                        self.add_source();
                    }
                    ui.add_space(4.0);
                    for source in &self.settings.sources {
                        let name = display_name(source);
                        let selected = self.view != View::Destinations
                            && self.selected_source == Some(source.id);
                        if ui.selectable_label(selected, name).clicked() {
                            self.selected_source = Some(source.id);
                            self.view = View::SourceSettings;
                            self.remove_source = false;
                            self.new_destination_draft = None;
                        }
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.heading("Destinationer");
                    if ui
                        .add_enabled(
                            !self.load_blocked,
                            egui::Button::new("+ Lägg till destination"),
                        )
                        .clicked()
                    {
                        self.add_destination();
                    }
                    ui.add_space(4.0);
                    for dest in &self.settings.destinations {
                        let selected = self.view == View::Destinations
                            && self.selected_destination == Some(dest.id);
                        if ui
                            .selectable_label(selected, dest.display_name())
                            .clicked()
                        {
                            self.selected_destination = Some(dest.id);
                            self.view = View::Destinations;
                            self.remove_destination = false;
                            self.new_destination_draft = None;
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.load_blocked {
                return;
            }
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(&mut self.view, View::Overview, "Översikt")
                    .clicked()
                {
                    self.new_destination_draft = None;
                }
                if ui
                    .selectable_value(&mut self.view, View::SourceSettings, "Inställningar")
                    .clicked()
                {
                    if self.selected_source.is_none() {
                        self.selected_source = self.settings.sources.first().map(|s| s.id);
                    }
                    self.remove_source = false;
                }
                if ui
                    .selectable_value(&mut self.view, View::Destinations, "Destinationer")
                    .clicked()
                {
                    if self.selected_destination.is_none() {
                        self.selected_destination =
                            self.settings.destinations.first().map(|d| d.id);
                    }
                    self.remove_destination = false;
                    self.new_destination_draft = None;
                }
            });
            ui.separator();

            match self.view {
                View::Overview => {
                    if self.settings.sources.is_empty() {
                        empty_sources_view(ui, || self.add_source());
                        return;
                    }
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for source in &self.settings.sources {
                                ui.push_id(source.id, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.heading(display_name(source));
                                        if ui.small_button("Inställningar").clicked() {
                                            self.selected_source = Some(source.id);
                                            self.view = View::SourceSettings;
                                            self.remove_source = false;
                                            self.new_destination_draft = None;
                                        }
                                    });
                                    source_controls(
                                        ui,
                                        &self.engine,
                                        source,
                                        &self.settings.destinations,
                                        self.engine.streaming(source.id),
                                        self.engine.recording(source.id),
                                    );
                                    ui.add_space(8.0);
                                });
                            }
                        });
                }
                View::SourceSettings => {
                    let Some(index) = self
                        .settings
                        .sources
                        .iter()
                        .position(|source| Some(source.id) == self.selected_source)
                    else {
                        empty_sources_view(ui, || self.add_source());
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
                    } = &mut self.settings;
                    let source = &mut sources[index];
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading(display_name(source));
                            source_controls(
                                ui,
                                &self.engine,
                                source,
                                destinations,
                                streaming,
                                recording,
                            );
                            if active {
                                ui.label(
                                    RichText::new(
                                        "Stoppa sändning och inspelning för att ändra inställningarna.",
                                    )
                                    .color(WARNING),
                                );
                            }
                            ui.add_enabled_ui(!active, |ui| {
                                outcome = ui
                                    .push_id(source.id, |ui| {
                                        source_settings(
                                            ui,
                                            source,
                                            destinations,
                                            &mut self.new_destination_draft,
                                            &devices,
                                        )
                                    })
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
                    if let Some(dest_id) = outcome.edit_destination {
                        self.selected_destination = Some(dest_id);
                        self.view = View::Destinations;
                        self.remove_destination = false;
                        self.new_destination_draft = None;
                    }
                    if removed {
                        self.settings.sources.remove(index);
                        self.selected_source = self
                            .settings
                            .sources
                            .get(index)
                            .or_else(|| self.settings.sources.last())
                            .map(|source| source.id);
                        self.remove_source = false;
                        self.new_destination_draft = None;
                    }
                    if outcome.changed || removed {
                        self.dirty = true;
                        self.engine.update(&self.settings.sources);
                    }
                }
                View::Destinations => {
                    let Some(index) = self
                        .settings
                        .destinations
                        .iter()
                        .position(|dest| Some(dest.id) == self.selected_destination)
                    else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui.heading("Lägg till en destination");
                            ui.label(
                                "Spara sändningsdestinationer globalt så kan de väljas av en eller flera ljudkällor.",
                            );
                            if ui.button("Lägg till destination").clicked() {
                                self.add_destination();
                            }
                        });
                        return;
                    };
                    let dest_id = self.settings.destinations[index].id;
                    let used_by: Vec<String> = self
                        .settings
                        .sources
                        .iter()
                        .filter(|source| source.destination_ids.contains(&dest_id))
                        .map(|source| display_name(source).to_owned())
                        .collect();
                    let in_use_streaming = self.destination_streaming(dest_id);
                    let mut changed = false;
                    let mut removed = false;
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let dest = &mut self.settings.destinations[index];
                            ui.heading(dest.display_name());
                            if used_by.is_empty() {
                                ui.weak("Används inte av någon ljudkälla ännu.");
                            } else {
                                ui.weak(format!("Används av: {}", used_by.join(", ")));
                            }
                            if in_use_streaming {
                                ui.label(
                                    RichText::new(
                                        "Stoppa pågående sändningar som använder den här destinationen för att ändra inställningarna.",
                                    )
                                    .color(WARNING),
                                );
                            }
                            ui.add_enabled_ui(!in_use_streaming, |ui| {
                                changed |= ui
                                    .push_id(dest.id, |ui| destination_settings(ui, dest))
                                    .inner;
                                ui.separator();
                                if self.remove_destination {
                                    ui.horizontal_wrapped(|ui| {
                                        let prompt = match used_by.len() {
                                            0 => "Ta bort den här destinationen?".to_owned(),
                                            1 => "Ta bort den här destinationen? Den tas även bort från 1 ljudkälla.".to_owned(),
                                            count => format!(
                                                "Ta bort den här destinationen? Den tas även bort från {count} ljudkällor."
                                            ),
                                        };
                                        ui.label(prompt);
                                        removed = ui.button("Ta bort").clicked();
                                        if ui.button("Avbryt").clicked() {
                                            self.remove_destination = false;
                                        }
                                    });
                                } else if ui.button("Ta bort destination").clicked() {
                                    self.remove_destination = true;
                                }
                            });
                        });
                    if removed {
                        self.settings.destinations.remove(index);
                        for source in &mut self.settings.sources {
                            source.destination_ids.retain(|id| *id != dest_id);
                        }
                        self.selected_destination = self
                            .settings
                            .destinations
                            .get(index)
                            .or_else(|| self.settings.destinations.last())
                            .map(|dest| dest.id);
                        self.remove_destination = false;
                    }
                    if changed || removed {
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

fn empty_sources_view(ui: &mut EguiUi, on_add: impl FnOnce()) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.heading("Lägg till en ljudkälla");
        ui.label("Välj en ljudingång och kanaler för sändning eller inspelning.");
        if ui.button("Lägg till ljudkälla").clicked() {
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

fn source_controls(
    ui: &mut EguiUi,
    engine: &Engine,
    source: &Source,
    destinations: &[Destination],
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
        let count = source.resolve_destinations(destinations).len();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    streaming || (!source.device.is_empty() && count > 0),
                    egui::Button::new(if streaming {
                        "Stoppa sändning"
                    } else {
                        "Starta sändning"
                    }),
                )
                .clicked()
            {
                engine.set_stream(source, destinations, !streaming);
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

#[derive(Default)]
struct SourceSettingsOutcome {
    changed: bool,
    edit_destination: Option<u64>,
}

fn source_settings(
    ui: &mut EguiUi,
    source: &mut Source,
    destinations: &mut Vec<Destination>,
    new_destination_draft: &mut Option<Destination>,
    devices: &[(String, String, usize)],
) -> SourceSettingsOutcome {
    let mut outcome = SourceSettingsOutcome::default();
    ui.heading("Ljudingång");
    egui::Grid::new("input-fields")
        .num_columns(2)
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
                    outcome.changed = true;
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

    ui.separator();
    ui.heading("Sändningsdestinationer");
    if source.destination_ids.is_empty() {
        ui.weak("Ingen destination vald för den här ljudkällan.");
    }
    let mut remove_slot = None;
    let mut replace_slot: Option<(usize, u64)> = None;
    for (slot_index, &dest_id) in source.destination_ids.iter().enumerate() {
        let Some(current) = destinations.iter().find(|dest| dest.id == dest_id) else {
            continue;
        };
        ui.push_id(("dest-slot", slot_index), |ui| {
            Frame::group(ui.style()).show(ui, |ui| {
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
                    if ui.small_button("Redigera").clicked() {
                        outcome.edit_destination = Some(dest_id);
                    }
                    if ui.small_button("Ta bort").clicked() {
                        remove_slot = Some(slot_index);
                    }
                });
                ui.weak(destination_summary(current));
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
        if new_destination_draft.is_none() && ui.button("+ Skapa ny destination").clicked() {
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
        Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong("Ny global destination");
            ui.push_id("new-dest-draft", |ui| {
                destination_settings(ui, draft);
            });
            ui.horizontal(|ui| {
                if ui.button("Skapa och välj").clicked() {
                    create = true;
                }
                if ui.button("Avbryt").clicked() {
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

    ui.separator();
    ui.heading("Lokal inspelning");
    egui::Grid::new("recording-fields")
        .num_columns(2)
        .show(ui, |ui| {
            outcome.changed |= field(ui, "Mapp", &mut source.recording.folder, false);
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
    outcome
}

fn destination_settings(ui: &mut EguiUi, destination: &mut Destination) -> bool {
    let mut changed = false;
    egui::Grid::new("destination-fields")
        .num_columns(2)
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
