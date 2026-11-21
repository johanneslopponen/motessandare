use eframe::egui::{Context, Id, Label, Ui};

fn mode_id() -> Id {
    Id::unique("contextual-help")
}

pub fn enabled(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(mode_id()).unwrap_or(false))
}

pub fn set_enabled(ctx: &Context, enabled: bool) {
    ctx.data_mut(|data| data.insert_temp(mode_id(), enabled));
}

/// Hover for a quick hint, or click to keep it open while reading.
pub fn tip(ui: &mut Ui, text: &'static str) {
    if enabled(ui.ctx()) {
        ui.push_id(text, |ui| {
            let menu = ui.menu_button("?", |ui| {
                ui.set_max_width(230.0);
                ui.add(Label::new(text).wrap());
            });
            if menu.inner.is_none() {
                menu.response
                    .on_hover_text(text)
                    .on_disabled_hover_text(text);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Event, PointerButton, Pos2, RawInput};

    #[test]
    fn clicking_a_hint_shows_only_the_local_text() {
        let ctx = Context::default();
        set_enabled(&ctx, true);
        let mut center = Pos2::ZERO;
        let _ = crate::test_ui::run_ui(&ctx, RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let group = ui.scope(|ui| tip(ui, "Välj din mikrofon."));
                center = group.response.rect.center();
            });
        });
        let mut visible = false;
        for pressed in [true, false, false] {
            let events = vec![
                Event::PointerMoved(center),
                Event::PointerButton {
                    pos: center,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ];
            let output = crate::test_ui::run_ui(
                &ctx,
                RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        ui.scope(|ui| tip(ui, "Välj din mikrofon."));
                    });
                },
            );
            visible |= output.shapes.iter().any(|s| {
                matches!(
                    &s.shape, egui::Shape::Text(t) if t.galley.text() == "Välj din mikrofon."
                )
            });
        }
        assert!(visible);
    }
}
