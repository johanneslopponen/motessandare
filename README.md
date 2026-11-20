# Mötessändare

Mötessändare är ett svenskspråkigt skrivbordsprogram för att direktsända möten från [Dalarnas Fridsförening](https://fridsforening.se/dalarna) till Icecast och samtidigt spela in lokalt. Varje ljudkälla har egna ingångskanaler, sändningsmål och inspelningsinställningar. Du kan också använda enbart inspelning eller sändning.

## Kom igång

1. Lägg till en ljudkälla och välj ljudenhet. Välj en kanal för mono eller två för stereo. Kontrollera på nivåmätaren att rätt signal kommer in.
2. För direktsändning, lägg till en destination med serveradress, monteringspunkt och inloggningsuppgifter. Välj destinationen för ljudkällan.
3. För inspelning, välj mapp och filformat under **Lokal inspelning**. Formaten är WAV, FLAC, Opus och MP3.
4. Gå till **Översikt**. Inställningarna sparas automatiskt efter en kort paus i redigeringen. Du kan starta sändning och inspelning var för sig och följa ljudnivå, inspelningstid och anslutna sändningsmål.

Välj **Kompakt översikt** för att se fler källor samtidigt. Nivåmätarens toppmarkör håller kvar korta ljudtoppar. Om signalen når klippgränsen visas en indikator som du kan återställa efter att du har kontrollerat nivån. **?** bredvid appnamnet visar små frågetecken vid reglagen. Peka eller klicka på dem för ett kort tips.

När du stoppar en inspelning och filen är färdigskriven kan du byta namn på den. **Spara namn** håller dialogen öppen så att du kan visa filen i Utforskaren eller Finder. **Klar** stänger dialogen.

Öppna **Lyssnarstatistik** med diagramikonen bredvid **?**. Vyn hämtar antalet anslutna lyssnare och lyssnartoppen per destination från Icecasts publika `/status-json.xsl`. Siffrorna uppdateras var tionde sekund medan vyn är öppen. Lyssnartoppen gäller den pågående sändningsanslutningen.

## Kör från källkod

Installera Rust med Cargo och FFmpeg med stöd för libopus, libmp3lame och protokollen `pipe` och `fd` (testat med FFmpeg 6.1 och senare). Appen kan använda FFmpeg från din `PATH` eller från samma katalog som den byggda programfilen. På Windows heter filen `ffmpeg.exe`. Linuxbygget behöver utvecklingspaketen för ALSA, OpenSSL och D-Bus.

```sh
cargo run --locked --release
```

## Sändningslösenord

Ange lösenordet i destinationens lösenordsfält och använd HTTPS för att kryptera ljud och inloggning under överföringen. Appen verifierar serverns certifikat och värdnamn. För HTTP-destinationer skickas ljud och inloggning i klartext. Ange serverns slutliga adress om den tidigare adressen har börjat omdirigera.

På macOS lagras lösenorden i Nyckelringen. På Linux används skrivbordets Secret Service, till exempel GNOME Keyring. På Windows skyddas lösenorden i inställningsfilen med användarkontots DPAPI. Äldre inställningar migreras när de öppnas. Om lösenordsförvaringen är låst behöver den låsas upp för att slutföra migreringen. Vid flytt till en annan dator behöver även lösenordsförvaringen återställas eller destinationernas lösenord anges på nytt. Äldre säkerhetskopior kan fortfarande innehålla klartextlösenord.

## Testa

```sh
cargo test --locked
```

För att även testa inspelning i fyra format, återanslutning och omdirigering mot lokala testservrar samt systemets lösenordsförvaring, kör `cargo test --locked -- --include-ignored`. Det kräver FFmpeg, lokala TCP-anslutningar och en tillgänglig lösenordsförvaring. Lösenordstestet använder tillfälliga testuppgifter.

## Paketera för macOS

På en Mac med Apple Silicon, Rust, Xcode Command Line Tools och `pkg-config`, kör:

```sh
bash scripts/package-macos.sh
```

Skriptet bygger appen och FFmpeg och lägger det zippade appaketet och motsvarande FFmpeg-källarkiv i `dist/`.

## Licens

Mötessändarens källkod har [MIT-licens](LICENSE). FFmpeg och övriga tredjepartsberoenden har egna licenser.
