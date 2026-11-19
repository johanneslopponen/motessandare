# Mötessändare: UI improvement suggestions

## Avstämning den 19 november 2026

Avstämningen gäller arbetskopian efter ändringarna i mötesöversikten och inspelningens stoppdialog. Numren hänvisar till den ursprungliga förslagslistan längre ned.

| Område | Genomfört | Delvis genomfört eller kvar |
|---|---|---|
| Navigation | 1.1, 1.3, 1.5. Flikarna ligger tillsammans och verktygsraden följer vyn. | 1.2 och 1.4. Från en ljudkälla leder **Redigera** fortfarande till hela destinationslistan. Det saknas en direkt väg tillbaka till samma källa. |
| Under mötet | 2.1, 2.2, 2.4. Toppfältet har kompakta ikoner, grupprubriker när de ryms, stoppbekräftelse med antal och förklaringar för inaktiva startknappar. 2.5 visar antalet anslutna mål och namnger mål med problem. | 2.3 visar inspelningstid. Sändningstid återstår. 2.6 lyfter fram saknad ljudingång, problem per mål och avslutade inspelningar med fel. En generell indelning av motorns meddelanden efter allvarlighetsgrad återstår. 2.7 är avprioriterad enligt användarens önskemål. |
| Nivåmätare | 3.1, 3.3, 3.4. Mätarna har dB-skala och färggradient med gränser i dB. | 3.2 och 3.5. Korta toppar kan fortfarande passera mellan två blickar. Ett kompakt läge för många källor återstår också. |
| Inställningar | 4.4. Lösenordet kan visas och döljas. Ljudenheterna uppdateras automatiskt. | 4.1–4.3, 4.5, 4.7 och 4.8 återstår. För 4.6 saknas kanalantal i enhetslistan. För 4.9 förklaras låsningen under drift, men den gäller hela formuläret. Filvisningen i stoppdialogen är genomförd. Mappval i inställningarna är en separat punkt. |
| Spara och återkoppla | 5.1–5.3. Ctrl/Cmd+S sparar, knappen visar **Sparat** och ändringsstatusen ligger intill. | 5.4 och 5.5. Ett tydligare felmeddelande med egen bakgrund och möjlighet att ångra borttagning återstår. |
| Utseende | Gemensamma knappar för huvudvyerna, röd bekräftelse för borttagning och en egen accentfärg för sändningsstopp. | Fasta formulärbredder och lokalt valda textstorlekar och avstånd finns kvar. Appen saknar manuellt temaval. |
| Tillgänglighet | De egendefinierade knapparna har roller, namn och synliga fokusmarkeringar. | Bygget aktiverar inte eframes AccessKit-stöd. Skärmläsarintegrationen behöver aktiveras och provas i den körande appen. |
| Prestanda och underhåll | Gränssnittet uppdateras mer sällan i vila. | Huvuddelen av gränssnittet ligger fortfarande i ui.rs. Stoppdialogen har en egen modul. |

**Förslag till nästa omgång.** Prioritera en kvarstående klippindikator på varje mätare (3.2), så att teknikern kan se en kort överstyrning i efterhand och kvittera den. Därefter skulle direktredigering av valt sändningsmål med återgång till källan (1.4) spara orientering när något behöver justeras.

## Ursprunglig förslagslista

Based on a review of [ui.rs](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs) (egui 0.31, single-column layout with three views: Översikt, källinställningar and Destinationer). Each group is ordered by impact. Effort: **S** = under 1 h, **M** = a few hours, **L** = needs engine or dependency changes.

---

## 1. Navigation and information architecture

| # | Suggestion | Why | Effort |
|---|---|---|---|
| 1.1 | **Put the views in one segmented control** (`Översikt` and `Destinationer` next to each other on the left). Keep only actions (`+ Ljudkälla`, `Spara`) on the right. | Right now `Destinationer` is a tab placed among the action buttons on the right ([L469-L551](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L469-L551)), so it looks like an action, not a view. | S |
| 1.2 | **Add a breadcrumb and back button in source settings**: `← Översikt / Ljudkälla 1`. | In SourceSettings the `Översikt` tab stays highlighted ([L476](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L476)), so it's unclear where you are and how to get back. | S |
| 1.3 | **Make the toolbar depend on the view.** Hide `Lägg till ljudkälla` in the Destinationer view, which already has its own `+ Lägg till destination`. | Two "add" buttons for different things on the same screen are confusing. | S |
| 1.4 | **Make "Redigera" on a destination slot open an inline editor** (or jump to and highlight that destination card, with a "Tillbaka till källa" link). | Today it switches to the Destinationer view and you lose context ([L1899](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L1899)). | M |
| 1.5 | **Add one `navigate(view)` helper.** | The reset pattern `remove_source = false; remove_destination = None; new_destination_draft = None` is copied about 6 times. If you forget one, transient state leaks between views. | S |

## 2. Live control (the core job: start, monitor, stop)

| # | Suggestion | Why | Effort |
|---|---|---|---|
| 2.1 | **Give the bulk header controls labels or counters**, for example `📡 2/3` and `⏺ 1/3`, or one toggle button per pill instead of separate start and stop icons. | The four icon-only buttons ([L327-L411](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L327-L411)) only explain themselves through tooltips. Users have to tell the "broadcast with slash" icon apart from "broadcast" while live. | S–M |
| 2.2 | **Ask for confirmation before "Stoppa alla inspelningar" and "Stoppa alla sändningar"** while anything is live. | One misclick ends a meeting recording. The per-source stop buttons sit right next to start in the same place, which carries the same risk. | S |
| 2.3 | **Show elapsed time on active cards**: `● Spelar in 00:42:17` and `Sänder 01:03:55`. | It's the most useful live information, and right now it's missing. Needs a `started_at` from the engine. | M (L if engine work grows) |
| 2.4 | **Explain why a button is disabled.** Use `on_disabled_hover_text`: "Välj en ljudenhet", "Lägg till en destination", "Välj inspelningsmapp". | The start buttons go grey without any reason ([L1605](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L1605), [L1625](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L1625)). Note that custom buttons use `Sense::hover()` when disabled, so hover text works. | S |
| 2.5 | **Show status per destination** on the card: the `N destinationer` pill expands to show each one with a dot (connected, reconnecting, error). Its tooltip should at least list the names. | When one of several Icecast targets fails, you can't see it from the overview. | M/L |
| 2.6 | **Style engine status by severity.** Today `engine.status()` is always grey secondary text ([L1583](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L1583)). Errors should be red and use an icon. | Errors look the same as information. | S–M |
| 2.7 | **Add a first-run checklist** on a card that isn't ready: ✓ Ljudenhet · ✗ Destination · ✗ Mapp, each linking to the right field. | Replaces the single warning line and guides setup. | M |

## 3. Level meters

| # | Suggestion | Effort |
|---|---|---|
| 3.1 | **Add scale ticks** (−60, −40, −20, −12, −6, 0 dB) under or on the bar. | S |
| 3.2 | **Add peak hold** (a thin marker that decays after about 1.5 s) and a **latching clip indicator** you click to reset. Today a short clip only flashes red for about 65 ms. | M |
| 3.3 | **Set the colour thresholds in dB** (green below −18, amber −18 to −3, red above −3) instead of linear 0.5 and 0.95 ([L1700-L1706](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L1700-L1706)). Linear 0.5 is −6 dBFS, so the amber zone is very narrow on a −60 dB scale. | S |
| 3.4 | **Use segmented or gradient meters** (classic LED style) so levels are readable at a glance from across the room. | M |
| 3.5 | **Add a compact card mode** (a single thin stereo meter and a one-line header) for setups with 4+ sources. | M |

## 4. Forms (source and destination settings)

| # | Suggestion | Why | Effort |
|---|---|---|---|
| 4.1 | **Add a `Bläddra…` folder picker** (`rfd` crate) and an `Öppna mapp` button next to *Mapp*. | Typing a path by hand is error-prone ([L2036](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L2036)). | M (new dependency) |
| 4.2 | **Validate fields inline**: server URL (scheme, host, port), mount (leading `/`), empty folder. Use a red border and a hint under the field. | Mistakes only show up when you start a stream. | M |
| 4.3 | **Add a `Testa anslutning` button** for each destination. | Lets you catch wrong credentials before the meeting. | L |
| 4.4 | **Add a show/hide toggle (👁) to the password field.** | Standard behaviour, and it helps catch typos. | S |
| 4.5 | **Use a segmented Mono/Stereo switch for channels**, then show only the channel pickers that are relevant. | The current left/right/"Ingen (mono)" combo logic is hard to follow ([L1795-L1840](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L1795-L1840)). | S–M |
| 4.6 | **Add a device-list refresh button (⟳) and show the channel count** in the device dropdown ("Focusrite 2i2 · 2 kanaler"). | Hot-plugged devices are a common case. | S |
| 4.7 | **Make destination cards collapsible**: a collapsed row shows name, summary and usage, and expands to the edit form. | The Destinationer list grows quickly, because every card shows 7 fields. | M |
| 4.8 | **Use segmented buttons instead of combo boxes** for short choice lists (Codec: Opus/MP3, Format: WAV/FLAC/Opus/MP3, Bitdjup 16/24). | One click instead of two, and every option is visible. | S |
| 4.9 | **Clarify what's locked while live**: lock only the fields that affect active outputs (for example, the name could stay editable) and show a lock icon in the section header. | Today the whole form goes grey ([L636](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L636)). | M |

## 5. Saving and feedback

| # | Suggestion | Effort |
|---|---|---|
| 5.1 | **Autosave** (debounced about 500 ms) and drop the Save button, *or* keep it but add **Ctrl/Cmd+S**. | S |
| 5.2 | **Add a short "Sparat ✓" toast or status-bar message** after saving. Today there's no success feedback, only errors. | S |
| 5.3 | **Move "Osparade ändringar"** to sit next to the Save button. With the right-to-left layout it currently ends up left of the `Destinationer` tab ([L543](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L543)). | S |
| 5.4 | **Show errors in a proper banner** with an icon, background colour and close button, instead of a plain warning label in the header ([L415](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L415)). | S |
| 5.5 | **Add an undo toast** ("Ljudkälla borttagen · Ångra") after deleting a source or destination, instead of the two-step inline confirm. | M |

## 6. Visual consistency

- **Mixed button styles:** custom `action_button` sits next to stock egui `ui.button` and `ui.small_button` ("Ta bort", "Avbryt", "Skapa och välj", "Redigera", "Läs in igen"). Add `ButtonVariant::Danger` and `ButtonVariant::Ghost` and use them everywhere. Destructive confirm buttons should be **filled red**. Right now the actual "Ta bort" confirm is the plainest button on screen ([L658](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L658), [L824](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L824)).
- **Stream-stop colour:** in the header, the stream "stop" button uses `record_accent` (red) ([L384](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L384)), so streaming and recording share a colour. Use neutral or teal for stop-stream.
- **Type scale:** font sizes are magic numbers (12, 12.5, 13.5, 14, 15.5, 16.5, 18). Define 4–5 constants (`CAPTION`, `BODY`, `TITLE_S`, `TITLE_M`, `TITLE_L`).
- **Spacing tokens:** `add_space(4/6/8/10)` is scattered everywhere. Use 2–3 named spacings.
- **Fixed widths:** inputs are fixed at 260 px, so on wide windows the forms hug the left edge. Use a max content width (about 720 px) centred, or let inputs stretch.
- **Manual theme toggle** (System / Ljust / Mörkt) in a small settings menu.

## 7. Accessibility and keyboard

- **The custom-painted buttons (`icon_button`, `action_button`, `tab_button`) don't call `response.widget_info(...)`.** Screen readers (AccessKit) don't get a role or label, and icon-only buttons get no name at all. They also draw **no focus ring** when `response.has_focus()`, so Tab navigation is invisible. This is the most important a11y fix (**S**).
- **Keyboard shortcuts:** for example Ctrl+S save, Ctrl+N new source, and Space/R for start/stop on the selected source. Show them in tooltips.
- **Contrast:** `text_disabled` on `btn_disabled_bg` in dark mode is about 2.6:1. That's fine for disabled controls, but don't use it for informational text (the Save icon uses it when nothing is dirty).

## 8. Performance and code health (indirect UI effects)

- `request_repaint_after(65 ms)` runs **unconditionally** ([L874](file:///d:/Dokument/ChatGPT/Mötessändare/src/ui.rs#L874)). Repaint fast only when a device is present or something is live. Otherwise use about 500 ms, which saves battery on laptops.
- `ui.rs` is about 2,300 lines. Split it into `theme.rs` (Palette, visuals, fonts), `widgets.rs` (buttons, pills, meter, icons) and `views/{overview,source,destinations}.rs`. That makes the improvements above much easier to iterate on.

---

## Suggested first batch (quick wins, about half a day)

1.1 · 1.2 · 1.5 · 2.2 · 2.4 · 3.1 · 3.3 · 4.4 · 4.8 · 5.1/5.2 · button variants (§6) · a11y widget info and focus ring (§7) · conditional repaint (§8)
