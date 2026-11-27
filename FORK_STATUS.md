# FORK_STATUS.md – Gesamtstand, Entscheidungen und offene Punkte (Honeydew-Fork)

Übergabe zwischen Sitzungen. **Zuerst lesen, wenn eine neue Sitzung beginnt.** `CLAUDE.md` hat die Arbeitsregeln und die
Architektur; diese Datei den Stand. Nach jedem Schritt hier nachtragen. Stand: Ende Tag 2 (Commit `e5e5780` und danach).

## 1. Worum es geht

Der/die Besitzer:in (keine Programmierkenntnisse, arbeitet **nur auf dem Handy, Android, Hochformat**, spricht Deutsch)
entwickelt diesen Fork von Honeydew (Open-Source-DAW in Rust, Vorbild FL Studio Mobile, GPL-3.0) zur besten DAW für
Android („gibt dort keine vergleichbar gute"). Erster Arbeitsbereich: der **GM Synth**. Wünsche: modernes, touchfreundliches
Layout im Hochformat, grafische reaktive Vorschauen (Filter, Envelopes, LFO, Oszillatoren), frei mappbare Envelopes/LFOs mit
Modulationsanzeige am Knopf, Custom-Wellenformen (später Wavetable), ein Farbsystem im Stil von Bitwig/Massive X (eigene
Umsetzung, übertragbar auf andere Module, später ans Theme anpassbar). Der Grundstil der FL-Mobile-Optik soll nicht
zu weit verloren gehen.

## 2. Arbeitsweise (Kurzfassung, Details in CLAUDE.md)

- Branch: `claude/honeydew-fork-setup-b6pc8o` (nur dort pushen, nie `main`, keine PRs ohne Wunsch). Fork: `OoTomEKKoO/honeydew`
  (öffentlich; Original `daijro/honeydew`, Version 0.5.5, unverändert = Commit `ed8fb6c`).
- **Builds bündeln und vorher fragen:** Jeder Push auf `claude/**` startet CI und den Android-Debug-Build. Vor einem solchen
  Push den/die Besitzer:in fragen, ob weitere Änderungen gewünscht sind, und die eigene Einschätzung sagen (genug für einen
  sinnvollen Test, oder zu viel auf einmal?). Doku-Commits mit `[skip ci]` (Markdown löst ohnehin nichts aus).
- Erklärungen in einfachem Deutsch, für das Handy formuliert (GitHub im Browser, APK installieren). Claude sieht die App nicht:
  Screenshots und konkrete Beschreibungen des/der Besitzer:in sind die Grundlage für UI-Arbeit.
- Vor jedem Push lokal: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  (ALSA-Entwicklerpakete installieren: `apt-get install -y libasound2-dev libxkbcommon-dev libwayland-dev libclang-dev`).
- APK holen: Link zum Lauf (`gh api repos/OoTomEKKoO/honeydew/actions/runs`), Artifact `honeydew-android-debug` (ZIP mit APK),
  installiert sich als „Honeydew Debug" über die vorige (fester Debug-Schlüssel `android/debug.keystore`). Ein Build braucht
  etwa 8 Minuten.
- Vergleich mit dem Original: Actions → „Android debug APK" → Run workflow, Feld `ref` (gleiche App-ID und Signatur).
- **Fallstrick:** GitHub schaltet Workflows auf Forks von Zeit zu Zeit ab (Banner „Workflows on this fork have been disabled",
  `gh api … dispatches` antwortet „Actions has been disabled"): der/die Besitzer:in muss im Browser auf den grünen Knopf
  „I understand my workflows, go ahead and re-enable them" tippen (github.com/OoTomEKKoO/honeydew/actions). Danach Build mit
  `gh api -X POST repos/OoTomEKKoO/honeydew/actions/workflows/android-debug.yml/dispatches -f ref=claude/honeydew-fork-setup-b6pc8o`.
- Referenztabelle des GM-Synth-Panels (`REFERENCE` in `src/panels/inst/gm_synth.rs`) pinnt Rechtecke je Seite: nach
  Layoutänderungen Zeilen von Hand anpassen (Reihenfolge, Widget-Art, Ränder, Größenregeln, Tinte) und die Rechtecke mit
  `tools/harvest-gmsynth-reference.py` aus den Testmeldungen übernehmen (zweimal laufen lassen).

## 3. Erledigt (alles im GM Synth, auf dem Handy getestet, soweit nicht anders vermerkt)

- **Zusatzfilter EXTRA** (Off/BP/Notch/Comb) – `crates/audio/src/gmsynth.rs` (`FILTER_EXTRA` 16300, `ladder`, `CombLines`).
- **FM RANGE** x1/x4/x8/x16 (`FM_RANGE` 32100, x1 = Original) – der Besitzer bestätigt: bringt hörbar etwas (Serum-artig körniger).
  Das „FM" des GM Synth ist Phasenmodulation (A bei φ_A + (B+1)·m·scale gelesen).
- **Speicherung kompatibel:** Zusatzeinstellungen im CSTM-Blob **hinter** den ersten 20 Bytes, nur wenn benutzt (`panels/cstm.rs`
  `GMSYNTH`, `grow()`); Songs ohne sie bleiben byte-gleich. *Ungeprüft:* ob FL Studio Mobile Songs mit eingeschaltetem
  EXTRA/FM RANGE fehlerfrei öffnet.
- **Zwei Tab-Leisten / zwei Seiten gleichzeitig** im Hochformat (`PartKind::Pager`, `build::pager`,
  `PanelState::{tab, tab2, dual_tabs, pick_tab}`, untere Leiste = Control-ID `LOWER_TABS` 1, Karte 18 U `GMSYNTH_PORTRAIT`,
  eine Seite 11 U `GMSYNTH_SINGLE`, Umschalter „One page"/„Two pages" im ≡-Menü der Karte, **nur pro Sitzung**, nicht gespeichert).
  Tab-Reihenfolge: **LEVEL, OSC, MOD, FILTER, EG, LFO, VOICES, FX** (Seitenindex 0..7).
- **Tabs auch mit zweitem Finger** (während eine Taste gehalten wird): `widgets/list.rs` `item_list_index` (Muster Issue #68).
- **Alle Slider im GM Synth sind Knobs.** Kompakte Menüs (`Leaf::compact`, `paint_compact` in `widgets/spinner.rs`: einzeilig,
  sonst Beschriftung über dem Wert, nie überlappend), `text_toggle` für KBD/VEL/MONO/AUTO, Menüs/Schalter in einer **Spalte am
  rechten Rand** (`with_side`), Knobs unten, Anzeigen oben (`picture`, `knobs`, `page`, `line` in `gm_synth.rs`).
- **Oszillator-Anzeigen** (OSC-Seite, A und B nebeneinander): Auswahlfeld per Tippen auf die Anzeige (`widgets::tap_picker`,
  Spinner mit `.draw`), ganzer Zyklus mit seitlichem Rand, pixelfein, A zeigt Ring/FM/Sync-Modulation (`curve_a`, B um die
  Mitte des A-Zyklus gelesen), B mit PHASE B, BALANCE als Deckkraft/Vorzeichen.
- **Filter-Anzeige** (FILTER-Seite): Frequenzgang log 20 Hz–20 kHz, −30…+24 dB, Cutoff-Linie und Resonanz-Ring, **Ziehen setzt
  CUTOFF (x) und RES (y)** (`gmsynth::filter_response`, per Zeitbereichstest gegen den echten Filter geprüft, auch mit Resonanz).
- **Envelope-Anzeigen** auf LEVEL, EG, MOD (`gmsynth::envelope_preview`: echte Exponentialkurven stufenweise, Breite je Stufe
  logarithmisch nach Zeit, echte Zeiten als Beschriftung, gegen die echte `Envelope` geprüft) und **LFO-Anzeige** (`lfo_preview`),
  nur Anzeigen. *Letzter Test steht aus:* bewegt sich die Envelope-Anzeige sichtbar mit den Knöpfen (war einmal zu starr).
- **Dunkle Popup-Menüs** (`ui/popup.rs`: `menuFill`-Panel, Labels `label_ink`, aktueller Eintrag `background`-Balken).
- **Farbsystem v1** (`theme::Accent`, `Ink::Accent`, `build::tint`): Osc gelb, Mod orange, Filter cyan, Level blau, Envelope grün,
  LFO violett, FX magenta; VOICES Kanalfarbe. Färbt Knob-Bogen/-Zahl, Menüwerte, Kurven. Kontrasttest. *Meinung des Besitzers steht aus.*
- Infrastruktur: Android-Debug-Workflow (`android-debug.yml`, mit optionalem `ref`), Windows/macOS-CI nur manuell, GPL-Merkzettel,
  Fork öffentlich, Commits als „Claude" ohne E-Mail, Link zur Sitzung in den Commit-Texten (Besitzer ok damit).

## 4. Offen (in der Reihenfolge, die ich empfehle; Besitzer hat die Wünsche genannt)

**A. Anbindung „Instrument → Oberfläche" (Spektrum und LFO-Phase)** – Voraussetzung für zwei Wünsche:
 - *Spektrum-Analyser als Hintergrund der Filter-Anzeige, reagiert auf den Filter.* Der Analyzer-Effekt existiert; Instrumente
   liefern bisher keine Analyse (`AudioRenderer::analysis(index)` ist für Instrumente leer; `crates/audio/src/device.rs`
   `Visual`-Seqlock-Slab pro Meter, Größe je Modultyp; `src/audio_engine/mod.rs` `analysis()`; App: `panels/fx/mod.rs`
   `live_analysis`, `post_feed`). Plan: `Instrument::analysis()` (Standard leer), GM Synth rechnet ein grobes Spektrum seiner
   Ausgabe (allokationsfrei) und die LFO-Phase, Engine/Meter-Slab für Instrumente erweitern.
 - *Laufender Punkt auf der LFO-Kurve synchron zur Modulation* (braucht die LFO-Phase).
**B. LFO-Erweiterung:** Sample & Hold, Sync-Modi (1/16 …), **„bei jeder Note ein neuer Zufallswert"** mit frei einstellbaren
 Grenzen (min/max). Neue CSTM-Einstellungen nach dem Muster `FILTER_EXTRA`/`FM_RANGE` (Feld in `cstm.rs`, `grow()`),
 Engine `gmsynth.rs` (`lfo_value`, der LFO ist bisher global, 3 Formen Sine/Saw/Square, Rate in Beats `GMSYNTH_RATE_BEATS`).
**C. Envelope-Punkte mit dem Finger ziehen** (Anzeigen in `gm_synth.rs`; `Draw` darf Params ändern: Beispiel `filter_display`).
 Die Zeit-Parameter-Gesetze sind exponentiell (`attack_rate`, `decay_rate`), Umkehrfunktion nötig.
**D. Frei mappbare Envelope(s) und LFO(s) im GM Synth** (zum Testen je 1; später per Kopie mehr) mit Ziel-Parameter und Amount,
 Ziele: Cutoff, Resonanz, Pitch, Osc-B-Pitch, Balance, Noise, Mod-Amount, Level, Pan; **Modulationsring am Knob** in der Farbe
 der Quelle, der sich mit dem Amount bewegt und von der Grundstellung abhebt (Bitwig-Prinzip). Neue Daten: Params hinter
 Index 74 (Vorbild: Honeydew-eigene SoundFont-Player-Params 6–11 in `params/instruments.rs`) – vorher prüfen, wie FLM-Laden/
 Speichern mit zusätzlichen Params umgeht. Das Modulator-Modul (Typ 1005) wird bewusst nicht angefasst.
**E. Custom-Wellenformen aus WAV-Dateien** für Osc A/B (Menü-Eintrag „Custom", Vorbild `three_osc::osc::Wave::custom`; Welle auf
 dem App-Thread bauen wie `gmsynth::prepare_setting`; Pfad im Song speichern; in ZIP-Export aufnehmen; Vorschau zeigt sie).
 Später: **Wavetable** (die Wellen-Vorschau wurde dafür vorbereitet).
**F. Gestaltung weiter:** farbiger Titel oder Streifen je Seite, Farbe der Quelle am Knopfring (siehe D), **Header sparen**
 (`CARD_HEADER` 1.5 U ist global, für den GM Synth schmaler?), Übertragung des Systems auf andere Module, Theme-Ableitung der
 Akzente (`Accent::color(&ThemeColors)` ist die Stelle), VOICES- und FX-Seite im neuen Stil verfeinern (FX noch fast wie früher),
 Ein-/Zwei-Seiten-Wahl speichern (Settings).
**G. Klang:** (1) Oszillator-Qualität vs. FL Mobile: zwei Saw-Wellen/FM klingen „kaputt", im Original-Honeydew genauso (bit-genau
 identisch zum Fork), FL Mobile klingt sauberer. Hypothesen: `gm_waveshapes::read` nimmt den nächsten Tabellenwert plus Dither
 (1024 Punkte) statt zu interpolieren, Aliasing bei hohen Tönen/starker FM, Detune. Vorgehen: Rauschabstand offline messen,
 Verbesserungen einzeln mit Messung und Hörtest. (2) Filter klingt „steril": Drive/Sättigung vor dem Filter, Resonanzverhalten,
 minimale analoge Unregelmäßigkeiten. Ausgang: Level-Spitzen von zwei Saw-Wellen liegen um 1,5 (Übersteuerung möglich).
**H. Sonstiges:** Upstream-Änderungen von `daijro/honeydew` gezielt übernehmen (cherry-pick; Repo mit `add_repo` anhängen); der
 Zeitplan-Workflow `lemon-effects.yml` des Originals läuft wöchentlich auf `main` (Abschalten angeboten, noch keine Antwort);
 Veröffentlichung später: GPL-Pflichten, Name/Branding prüfen (siehe CLAUDE.md); FL-Mobile-Kompatibilität der Songs prüfen.

## 5. Offene Fragen / noch nicht beurteilt vom Besitzer

- Letzte APK (Commit `3587d86`): Menüs in der Spalte am rechten Rand (besonders LFO, dort am niedrigsten), Envelope-Anzeigen
  bewegen sich, FM-Anzeige bei tiefem Pitch von B symmetrisch, Farben.
- Gefällt das Farbsystem? Farbige Titel/Streifen nötig? Zu bunt oder zu wenig?
- Dunkle Popup-Menüs: Farbe ok?
- Reihenfolge wie empfohlen (A+B zusammen, dann C, dann D …) oder anders?

## 6. Bekannte Einschränkungen

- Die Oszillator-FM-Anzeige ist ein **Schnappschuss** (im Klang sieht jeder A-Zyklus einen anderen Teil von B); PITCH B springt in
  Halbtonstufen (FINE B ist stetig). Starke FM (x16) zeigt stauchende Spitzen – das ist der echte Verlauf.
- Envelope-Anzeige: Zeitachse nur grob proportional; Filter-Envelope (Seite FILTER: ATTACK/DECAY/AMOUNT) hat noch keine Anzeige.
- Ein-/Zwei-Seiten-Wahl ist nicht gespeichert. Push-Benachrichtigungen kamen beim Besitzer nicht an (Einstellung dort).
- Windows/macOS/iOS/Web: Code kompiliert weiter (CI Linux: wasm-Check), wird aber nicht getestet.


---

# Anhang: ursprüngliche Roadmap-Texte (Tag 1/2, zur Nachvollziehbarkeit)

## Roadmap (vereinbart mit dem/der Besitzer:in, Android-Hochformat im Fokus)

Jeder Schritt = eigener Build zum Testen auf dem Handy, danach Rückmeldung. UI muss zum Stil von FL Mobile/Honeydew
passen (Farben, Schriften, Abstände der bestehenden Panels; nichts Überlappendes, touchfreundliche Flächen).
**Entscheidung: alles direkt im GM Synth, das Modulator-Modul (Typ 1005) wird vorerst nicht angefasst.** Die
GM-Synth-Stimmen haben ihre eigenen Envelopes: Envelopes starten bei jedem Anschlag neu und gelten pro Stimme.

1. **GM-Synth-Layout im Hochformat.** `KIND`/`EXTRA` überlappen (Spinner-Beschriftung vs. Pfeile, EXTRA-Zeile zu eng:
   Reihe auf FILTER hat 3 Zeilen in 6.24 U) – reparieren. Neu: **zwei Tabs gleichzeitig übereinander** anzeigen
   (z. B. MOD + OSC oder zwei beliebige), welche beiden sichtbar sind, lässt sich durchschalten/wählen. Karte im
   Hochformat entsprechend höher (Vorbild: `SOUNDFONT_PORTRAIT` in `src/ui/rack.rs`; `ui_height(type_id)` ist bisher
   nur pro Typ statisch, `card_height(module)` bekommt das Modul).
2. **Wellenform-Vorschau pro Oszillator (A und B), hochwertig und reaktiv** (folgt Auswahl, später Custom-Welle, Pitch/
   Phase B/Balance), im Stil des Moduls (Display via `.draw(fn)` wie in `src/panels/inst/three_osc.rs`; Daten aus
   `crates/audio/src/gm_waveshapes.rs`). Vorbereitung für späteren Wavetable-Einbau.
3. **Grafische Vorschau für die vorhandenen Envelopes** (Level, Filter, Mod, EG) und den LFO, **mit Fingerbedienung**:
   Punkte (Attack, Decay, Sustain, Release) direkt ziehen statt Slider. Eine `Draw`-Funktion darf Params ändern und
   melden (`src/panels/mod.rs` `Draw`), das geht also ohne Formatänderung.
4. **Neuer Tab für frei mappbare Envelopes und LFOs** im GM Synth (wie 3x Osc, mit ziehbaren Punkten): mehrere
   Envelopes (starten bei jedem Anschlag) und LFOs, jeder mit Ziel-Parameter (aus den GM-Synth-Parametern) und Amount.
   LFO-Formen auch **Sample & Hold**, mit Sync-Modi (1/16 usw.) und Trigger-Modi (frei / bei jeder Note neu).
   Entschieden: zum Testen **je 1 Envelope und 1 LFO** (später per Kopie erweiterbar); Ziele: Cutoff, Resonanz, Pitch,
   Osc-B-Pitch, Balance, Noise, Mod-Amount, Level, Pan. Neue Daten brauchen Platz im Song: Params
   hinter Index 74 (Vorvorbild: Honeydew-eigene SoundFont-Player-Params 6–11 in `src/params/instruments.rs`) – vorher
   prüfen, wie FLM-Laden/Speichern mit zusätzlichen Params umgeht. Engine: `crates/audio/src/gmsynth.rs`, ein LFO ist
   bisher global.
5. **Custom-Wellenformen aus WAV-Dateien** für Osc A und B (Vorbild: `three_osc::osc::Wave::custom`; Welle auf dem
   App-Thread bauen wie `gmsynth::prepare_setting`; Pfad im Song speichern; in ZIP-Export aufnehmen).
6. **Klang des Filters** (wirkt „steril"): Drive/Sättigung vor dem Filter, Resonanzverhalten, minimale analoge
   Unregelmäßigkeiten; nach Gehör des/der Besitzer:in.

Erledigt: GM Synth `EXTRA`-Filter (Off/BP/Notch/Comb), Android-Debug-Build per GitHub Actions.
Schritt 1 (Layout) auf dem Handy getestet: UI gut; Wunsch nach zwei Tab-Leisten umgesetzt (`PartKind::Pager` +
`build::pager`, `PanelState::{tab, tab2, dual_tabs, pick_tab}`, untere Leiste `LOWER_TABS` = Control-ID 1,
`GMSYNTH_PORTRAIT` 16 U in `src/ui/rack.rs`), FILTER mit 7 Spalten (EXTRA eigene Spalte). Wartet auf Handy-Test.
Schritt 2 gebaut, wartet auf Handy-Test: Wellenform-Vorschau pro Oszillator auf der OSC-Seite (`wave_a`/`wave_b` in
`src/panels/inst/gm_synth.rs`, Daten `gmsynth::preview_cycle`; folgt Shape-Auswahl, BALANCE-Anteil, PHASE B).
Klang-Befund: Besitzer:in meldete, zwei Saw-Wellen im GM Synth (Regler ziehen) klingen „kaputt". Bit-genauer Vergleich
(Original ed8fb6c vs. Fork, über `SongAudioEngine` und direkt, alle Osc-Mod-Arten × 3 Filter, Saw/Saw bei vielen
FINE-B/BALANCE-Werten, EXTRA Off) ist identisch; die unveränderte Original-Debug-App und die Original-Honeydew-App klingen
genauso, **FL Studio Mobile klingt dort sauberer**. Also ein bestehender Unterschied zum Vorbild, kein Fehler des Forks.
Das GM-Synth-„FM" ist Phasenmodulation (A wird bei φ_A + (B+1)·m gelesen). Vergleichsbuild eines beliebigen Commits:
Actions → Android debug APK → Run workflow, Feld `ref` (gleiche App-ID und Signatur, installiert über die Debug-App).

Rückmeldung zu Schritt 2 (Screenshot): Tabs funktionieren; Wellen-Vorschau als Entwurf ok, soll hochauflösender, zentriert,
ohne Anstoßen am Rand sein und auf FM reagieren; Layout verschwenderisch. Umgesetzt: `gmsynth::FM_RANGE` (CSTM 32100,
x1/x4/x8/x16, x1 = Original), Vorschau mit Pixelauflösung und `curve_a` (Ring/FM/Sync).
Schritt A (Knobs, Wellen-Auswahl per Tippen, Ein-/Zwei-Seiten-Schalter) auf dem Handy getestet: „sieht gut aus".
Offene Wünsche daraus: Platz besser nutzen (Knobs größer, Pfeil-Menüs kompakter), Header sparen, Filter-Vorschau, Envelopes/LFOs.
Schritt A2 gebaut (wartet auf Handy-Test): kompakte Menüs (`Leaf::compact`/`.compact()`, `Control::compact`,
`paint_compact` in `src/ui/widgets/spinner.rs`: Kasten mit Label über Wert, Tippen irgendwo öffnet die Liste), niedrige
Menü-Reihe (`MENUS` = 2 U) unter größeren Knobs; FILTER-Seite mit Frequenzgang-Anzeige (`gmsynth::filter_response`,
per Zeitbereichsmessung des echten Filters geprüft; Ziehen setzt CUTOFF (x) und RES (y)); Envelope-Anzeigen auf LEVEL,
EG und MOD (`gmsynth::envelope_preview` lässt die echte `Envelope` mit 1 kHz laufen) und LFO-Anzeige (`lfo_preview`),
alle nur Anzeige (Punkte ziehen und freies Mapping noch offen, siehe Roadmap 3 und 4). Karte 18 U.
Rückmeldung zum Test: FM RANGE bringt hörbar etwas; die Auswahlmenüs (Popups) waren hell/weiß und passten nicht zur UI ->
in `src/ui/popup.rs` jetzt `menuFill`-Panel mit `text`-Labels (Honeydew-Änderung, FL Mobile ist invertiert), der aktuelle Eintrag
ein `background`-Balken unter vollem `text`; Hilfen `popup::panel_fill` und `popup::label_ink` (Tests nutzen sie).
Hinweis: Die A-Vorschau bei starker FM (x16) wirkt „gestaucht" – das ist der echte FM-Verlauf (B ist spitz), kein Zeichenfehler.
Entschieden: Modulationsanzeige am Knob = farbiger Ring, der sich mit dem Amount bewegt und von der Grundstellung abhebt
(kommt mit Schritt B, den Envelopes/LFOs und dem Mapping). Header (CARD_HEADER 1.5 U, global) noch nicht verkleinert.

Feedback Tag 2 (Screenshot FILTER/OSC): (1) Tab-Reihenfolge logisch: LEVEL, OSC, MOD, FILTER, EG, LFO, VOICES, FX
(umgesetzt, Seitenindex 0..7 in dieser Reihenfolge). (2) FILTER: Anzeige höher, Knobs größer, Menüs/KBD klein in einer
Zeile (umgesetzt: einzeilige kompakte Menüs `paint_compact`, `text_toggle` für KBD/VEL/MONO/AUTO, `MENUS` = 1.4 U);
Resonanz der Anzeige per Zeitbereichstest gegen den echten Filter geprüft (stimmt, war nur zu flach: Bereich jetzt
−30…+24 dB). WUNSCH OFFEN: Spektrum-Analyser als Hintergrund der Filter-Anzeige, der auf den Filter reagiert (der
Analyzer-Effekt existiert; Instrumente liefern bisher keine Analyse: `AudioRenderer::analysis` ist für Instrumente leer,
`device.rs` `Visual`-Slab pro Meter, Größe je Effekttyp; für den GM Synth bräuchte es `Instrument::analysis()` + Anbindung
in `audio_engine/mod.rs` und `live_analysis`). (3) OSC: Anzeige beschneidet an den Rändern, ruckelt bei kleiner Frequenz von
B, Knobs größer (umgesetzt: genau ein ganzer Zyklus pro Anzeige, seitlicher Rand; Ruckeln kommt von den Halbtonstufen
von PITCH B). (4) MOD/LEVEL/EG: Anzeige höher; Attack wurde erst ab ~20 % sichtbar (umgesetzt: `envelope_preview` rechnet die
echten Exponentialkurven stufenweise mit festen Breitenanteilen A 22 %/D 30 %/Halten 13 %/R 35 %, echte Zeiten als Label,
gegen die echte `Envelope` geprüft). WUNSCH OFFEN LFO: Sample & Hold, Sync-Modi, „bei jeder Note neuer Zufallswert" mit frei
einstellbaren Grenzen, laufender Punkt auf der Kurve synchron zur Modulation (braucht die LFO-Phase aus der Engine, siehe
Analyse-Anbindung oben). WUNSCH OFFEN Farbsystem: übersichtliche, zum Theme passende, vordefinierte Kontraste (Vorbild
Massive X von Native Instruments), damit die Module nicht monoton wirken, ohne die FL-Mobile-Optik ganz zu verlieren.

7. **Oszillator-Klangqualität im GM Synth angleichen an FL Mobile** (später): Hypothesen, noch ungeprüft: der Tabellen-
   lesezugriff `gm_waveshapes::read` nimmt den nächsten Tabellenwert plus Dither (1024 Punkte) statt zu interpolieren;
   Aliasing der bandbegrenzten Tabellen bei hohen Tönen und starker FM; Detune zweier Saw-Wellen. Vorgehen: Spektrum/
   Rauschabstand offline messen (Test rendert Saw/Saw), Verbesserungen (Interpolation, mehr Tabellenpunkte,
   Oversampling bei FM) einzeln mit Messung und Hörtest des/der Besitzer:in.
