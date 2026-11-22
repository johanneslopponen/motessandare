use eframe::egui::{self, Align, Context, Id, Layout, Ui};
use std::io::{Error, ErrorKind};

pub fn file_hint(error: &Error) -> &'static str {
    match error.kind() {
        ErrorKind::PermissionDenied => "Kontrollera att du har åtkomst till filen och mappen.",
        ErrorKind::StorageFull => "Disken är full. Frigör utrymme och försök igen.",
        ErrorKind::NotFound => "Kontrollera att filen eller mappen finns kvar.",
        ErrorKind::NotADirectory | ErrorKind::IsADirectory => {
            "Kontrollera att sökvägen pekar på rätt fil och mapp."
        }
        ErrorKind::ReadOnlyFilesystem => "Disken är skrivskyddad.",
        _ => "Kontrollera sökvägen, åtkomsten till mappen och ledigt diskutrymme.",
    }
}

/// Returns true when the operator dismisses this message.
pub fn message(ui: &mut Ui, text: &str) -> bool {
    let accent = ui.visuals().warn_fg_color;
    egui::Frame::new()
        .fill(accent.gamma_multiply(0.08))
        .stroke(egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.5)))
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    let dismissed = ui
                        .button("Dölj")
                        .on_hover_text("Dölj meddelandet")
                        .clicked();
                    ui.vertical(|ui| {
                        ui.add(egui::Label::new(text).wrap());
                    });
                    dismissed
                })
                .inner
            })
            .inner
        })
        .inner
}

// A dismissal belongs to one control and one cause. A change of cause or
// a recovery makes the next occurrence visible again.
fn visible(ctx: &Context, id: Id, text: Option<&str>) -> bool {
    ctx.data_mut(|data| {
        let hidden = data.get_temp::<Id>(id);
        if hidden.is_some() && hidden == text.map(Id::unique) {
            return false;
        }
        data.remove::<Id>(id);
        text.is_some()
    })
}

pub fn condition(ui: &mut Ui, id: Id, text: Option<&str>) {
    show_if_needed(ui, id, text, message);
}

pub fn show_if_needed(
    ui: &mut Ui,
    id: Id,
    text: Option<&str>,
    show: impl FnOnce(&mut Ui, &str) -> bool,
) {
    if visible(ui.ctx(), id, text)
        && let Some(text) = text
        && show(ui, text)
    {
        ui.data_mut(|data| data.insert_temp(id, Id::unique(text)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{CentralPanel, Event, PointerButton, Pos2, RawInput, Rect, Shape, Vec2};

    #[test]
    fn long_message_wraps_and_can_be_dismissed_independently_until_recovery() {
        let ctx = Context::default();
        let first = Id::unique("first");
        let second = Id::unique("second");
        let text = "Ljudet kunde inte skickas till Icecast. Kontrollera nätverket och inloggningsuppgifterna.";
        let draw = |events, reason: Option<&str>| {
            crate::test_ui::run_ui(
                &ctx,
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(295.0, 400.0))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    CentralPanel::default().show(ui, |ui| {
                        condition(ui, first, reason);
                        condition(ui, second, Some(text));
                    });
                },
            )
        };
        let frame = draw(vec![], Some(text));
        let bounds = |frame: &egui::FullOutput, label: &str| -> Vec<Rect> {
            frame
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    Shape::Text(t) if t.galley.text() == label => Some(t.visual_bounding_rect()),
                    _ => None,
                })
                .collect()
        };
        let messages = bounds(&frame, text);
        let buttons = bounds(&frame, "Dölj");
        assert_eq!(messages.len(), 2);
        assert_eq!(buttons.len(), 2);
        assert!(messages[0].height() > 30.0);
        assert!(messages[0].right() < buttons[0].left());
        assert!(messages[0].bottom() < messages[1].top());
        let pos = buttons[0].center();
        for pressed in [true, false] {
            draw(
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        pressed,
                        button: PointerButton::Primary,
                        modifiers: Default::default(),
                    },
                ],
                Some(text),
            );
        }
        assert_eq!(bounds(&draw(vec![], Some(text)), text).len(), 1);
        assert!(visible(&ctx, first, Some("En annan orsak")));
        ctx.data_mut(|data| data.insert_temp(first, Id::unique(text)));
        draw(vec![], None);
        assert_eq!(bounds(&draw(vec![], Some(text)), text).len(), 2);
    }
}
