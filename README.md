# Mötessändare

Mötessändare är ett svenskspråkigt skrivbordsprogram för att direktsända möten till Icecast och samtidigt spela in lokalt. Varje ljudkälla har egna ingångskanaler, sändningsmål och inspelningsinställningar. Du kan också använda enbart inspelning eller sändning.

## Kom igång

1. Lägg till en ljudkälla och välj ljudenhet. Välj en kanal för mono eller två för stereo. Kontrollera på nivåmätaren att rätt signal kommer in.
2. För direktsändning, lägg till en destination med serveradress, monteringspunkt och inloggningsuppgifter. Välj destinationen för ljudkällan.
3. För inspelning, välj mapp och filformat under **Lokal inspelning**. Formaten är WAV, FLAC, Opus och MP3.
4. Gå till **Översikt**. Inställningarna sparas automatiskt efter en kort paus i redigeringen. Du kan starta sändning och inspelning var för sig och följa ljudnivå, inspelningstid och anslutna sändningsmål.

Välj **Kompakt översikt** för att se fler källor samtidigt. Nivåmätarens toppmarkör håller kvar korta ljudtoppar. Om signalen når klippgränsen visas en indikator som du kan återställa efter att du har kontrollerat nivån. **?** bredvid appnamnet öppnar användarguiden.

När du stoppar en inspelning och filen är färdigskriven kan du byta namn på den. **Spara namn** håller dialogen öppen så att du kan visa filen i Utforskaren eller Finder. **Klar** stänger dialogen.

## Kör från källkod

Installera Rust med Cargo och en FFmpeg-version med stöd för Icecast, libopus och libmp3lame. Appen kan använda FFmpeg från din `PATH` eller från samma katalog som den byggda programfilen. På Windows heter filen `ffmpeg.exe`.

```sh
cargo run --locked --release
```

## Testa

```sh
cargo test --locked
```

För att även testa inspelning i fyra format och återanslutning mot en lokal testserver, kör `cargo test --locked -- --include-ignored`. Det kräver FFmpeg och tillgång till lokala TCP-anslutningar.

## Paketera för macOS

På en Mac med Apple Silicon, Rust, Xcode Command Line Tools och `pkg-config`, kör:

```sh
bash scripts/package-macos.sh
```

Skriptet bygger appen och FFmpeg och lägger det zippade appaketet och motsvarande FFmpeg-källarkiv i `dist/`.

## Licens

Mötessändarens källkod har [MIT-licens](LICENSE). FFmpeg och övriga tredjepartsberoenden har egna licenser.
