use eframe::egui::{Context, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape, vec2};

/// Clicks a rendered button and returns the frame in which the click was released.
pub(crate) fn click_button(
    ctx: &Context,
    label: &str,
    mut show: impl FnMut(&Context),
) -> FullOutput {
    let mut frame = |events| {
        ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1600.0))),
                events,
                ..Default::default()
            },
            &mut show,
        )
    };

    // Allow newly opened windows and wrapped controls to settle before hit testing.
    frame(vec![]);
    let layout = frame(vec![]);
    let mut labels = layout.shapes.iter().filter_map(|clipped| {
        if let Shape::Text(text) = &clipped.shape
            && text.galley.text() == label
            && clipped
                .clip_rect
                .contains(text.visual_bounding_rect().center())
        {
            Some(text.visual_bounding_rect().center())
        } else {
            None
        }
    });
    let pos = labels
        .next()
        .unwrap_or_else(|| panic!("missing button {label:?}"));
    assert!(labels.next().is_none(), "ambiguous button {label:?}");

    let button = |pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    frame(vec![Event::PointerMoved(pos), button(true)]);
    frame(vec![button(false)])
}
