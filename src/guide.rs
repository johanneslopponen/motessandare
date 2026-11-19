use eframe::egui::{self, Context, ScrollArea};

const TOPICS: &[(&str, &str)] = &[
    (
        "Välj ljudingång",
        "Klicka på Lägg till ljudkälla i Översikt. Välj din mikrofon eller ditt ljudkort under Ljudenhet. För mono räcker en kanal. För stereo väljer du även en höger kanal. Tala i mikrofonen och kontrollera att nivåmätaren reagerar. Inställningarna sparas automatiskt en kort stund efter din senaste ändring.",
    ),
    (
        "Sänd till Icecast",
        "Öppna ljudkällans Inställningar. Under Sändningsdestinationer kan du välja ett sparat mål eller skapa ett nytt med uppgifterna från din Icecast-server. Klicka på Starta sändning och kontrollera hur många mål som är anslutna.",
    ),
    (
        "Spela in en fil",
        "Välj mapp och filformat under Lokal inspelning i ljudkällans Inställningar. Klicka på Starta inspelning. Du kan spela in och sända samtidigt eller bara spela in.",
    ),
    (
        "Följ mötet",
        "Varje kort i Översikt styr en ljudkälla. Knapparna högst upp styr alla källor tillsammans. Välj Kompakt översikt om du vill se fler källor samtidigt. Markören i nivåmätaren visar den senaste toppnivån en kort stund. Visas Klippning har ljudet nått maxnivån. Sänk ingångsnivån och klicka på Återställ för att upptäcka nästa topp.",
    ),
    (
        "Avsluta och hitta filen",
        "Klicka på Stoppa inspelning. När filen är färdigskriven kan du ändra filnamnet, spara namnet och visa filen i filhanteraren. Klicka på Klar för att stänga dialogen. En pågående sändning stoppas separat.",
    ),
];

/// Returns true when the user dismisses the guide.
pub fn show(ctx: &Context) -> bool {
    let response = egui::Modal::new(egui::Id::new("user-guide")).show(ctx, |ui| {
        let viewport = ctx.screen_rect();
        ui.set_width((viewport.width() - 56.0).clamp(200.0, 460.0));
        ui.heading("Så använder du Mötessändare");
        ui.separator();
        ScrollArea::vertical()
            .max_height((viewport.height() - 145.0).clamp(90.0, 540.0))
            .show(ui, |ui| {
                for &(heading, body) in TOPICS {
                    ui.strong(heading);
                    ui.label(body);
                    ui.add_space(6.0);
                }
            });
        ui.separator();
        ui.button("Stäng guiden").clicked()
    });
    response.inner || response.should_close()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_fits_smallest_window_and_closes_with_escape() {
        let ctx = Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(560.0, 480.0));
        for escape in [false, false, true] {
            let mut input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            if escape {
                input.events.push(egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let _ = ctx.run(input, |ctx| assert_eq!(show(ctx), escape));
        }
        let bounds = ctx
            .memory(|m| m.area_rect(egui::Id::new("user-guide")))
            .unwrap();
        assert!(screen.contains_rect(bounds), "{bounds:?}");
    }
}
