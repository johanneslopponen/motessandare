use eframe::egui::{
    Context, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Shape, Ui, vec2,
};

/// Runs a headless egui pass and clears unapplied texture deltas for tests.
pub(crate) fn run_ui(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// Clicks a rendered button and returns the frame in which the click was released.
pub(crate) fn click_button(
    ctx: &Context,
    label: &str,
    mut show: impl FnMut(&mut Ui),
) -> FullOutput {
    let mut frame = |events| {
        run_ui(
            ctx,
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
