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

## 3b. Gebaut und getestet, aber NOCH NICHT als APK gebaut (wartet auf Rückfrage vor dem Build)

- Kleine Korrekturen aus dem Feedback zur APK `3587d86`: (1) Envelope-Anzeige: eine Stufe mit der Zeit 0 ist jetzt senkrecht
  (`env_weight` ab 0, feste Gesamtbreite, danach flache Linie), Breite folgt der Zeit; (2) OSC-Anzeige: bei langsamem B öffnet
  sich das Fenster auf bis zu 4 Zyklen von A (`curve_a`, `MAX_A_CYCLES`), weil ein tiefer Modulator über viele Zyklen läuft
  (Ursache für „kaum Einfluss": ein Zyklus sah fast unmoduliert aus); (3) LFO-Seitenspalte: Menüs oben, AMOUNT-Knopf unten.
- **Punkt A erledigt:** `Instrument::analysis()`, `InstrumentModule::analysis`, `audio_engine/mod.rs` `analysis()` liefert für
  Instrumente; `crates/audio/src/spectrum.rs` (`SpectrumProbe`, FFT 4096, 64 Bänder, allokationsfrei); GM Synth meldet
  `LIVE_LFO`, `LIVE_PHASE`, Spektrum (`LIVE_BANDS`); Filter-Anzeige zeichnet das Spektrum hinter der Antwortkurve, LFO-Anzeige
  einen laufenden Punkt (`panels::live_analysis`, sichtbar `pub(crate)`).
- **Punkt B erledigt:** LFO-Modi (`LFO_MODE` 65100: Shape/S&H/Note, `LFO_LOW` 65200, `LFO_HIGH` 65300) im CSTM-Blob ab Byte 28
  (`grow()` füllt Lücken mit Standardwerten), Engine in `gmsynth.rs` (globaler LFO: S&H neuer Wert bei jedem Zyklus, Note neuer
  Wert bei jeder Note), LFO-Seite mit MODE-Menü und LOW/HIGH-Knöpfen (nur bei Zufallsmodi sichtbar), Anzeige mit Band und Stufen.
  *Noch nicht gebaut:* Sync-Modi über das vorhandene tempo-synchrone RATE hinaus (freie Hz-Rate), LFO pro Stimme.

## 3c. Feedback zur APK `a053c5b` und Umsetzung (gebaut und getestet, NOCH NICHT als APK; wartet auf Rückfrage)

- LFO-Seite: AMOUNT-Knopf winzig -> AMOUNT steht jetzt in der Knopfreihe unten (mit KEY, LOW, HIGH), die Seitenspalte hat 5 Menüs
  (MODE, SHAPE, DEST, RATE, SYNC) bei ~1.1 U Höhe (Abstand der Seitenspalte 0.06 U).
- Spektrum feiner und reaktiver: `spectrum.rs` FFT 2048, Hop 512, 96 Bänder, schnellere Glättung, schmale Bänder interpoliert;
  Anzeige als einzelne Balken mit hellerer Kappe (`filter_display`), Neuzeichnen alle 16 ms (`live_analysis_fast`).
- Filter bewegt sich mit der Modulation: `LIVE_CUTOFF` (mittlerer echter Cutoff der klingenden Stimmen inkl. Envelope, LFO,
  KBD), zweite, hellere Kurve und Linie in der Anzeige; die Kurve des Knopfwerts bleibt als Referenz.
- Envelope-Beschriftung überlappte bei kurzen Zeiten -> eine Zeile „A … D … S …% R …".
- LFO-Rate: neues Menü SYNC (`LFO_SYNC` 65400: Orig., 1/32, 1/16T, 1/16, 1/8T, 1/16., 1/8, 1/4T, 1/8., 1/4, 1/2, 1 Bar, 2/4/8 Bars;
  `Orig.` = das bisherige RATE-Menü, dessen Stufen in Viertelschlägen beschriftet waren: „1/4" = ein Viertelschlag = 1/16)
  und Schalter KEY (`LFO_KEYSYNC` 65500: jede Note startet den LFO-Zyklus neu, auch beim laufenden Song). CSTM-Felder ab Byte 40/44.

> Regel seit Tag 3: Pushen und Bauen ist erlaubt. Nach Feedback einmal den Plan abstimmen, dann bis zur APK durchziehen.

## 3d. Mod-Matrix (Schritt 1, lokal committet, NICHT gepusht, keine APK)

- Engine (`crates/audio/src/gmsynth.rs`): 4 freie Routen (Quelle LFO oder EG, Ziel aus 17 weichen Reglern, Betrag
  bipolar, Mitte = 0). Die Routen wirken auf eine Kopie der Parameter pro 32-Sample-Block (keine Speicher-Allokation,
  kaum CPU). Der EG liefert den Wert der neuesten klingenden Stimme. Die alten Einzelziele (`LFO Dest`, `EG Dest`) bleiben.
- Datei: 12 neue CSTM-Felder ab Byte 48 (`MOD_BASE` 66000), nur geschrieben, wenn benutzt (alte Songs unverändert).
- Oberfläche: neue Seite MATRIX (nach LFO) mit 4 Zeilen FROM / TO / AMOUNT. Prototyp.
- Schritt 2 (gebaut, Tests grün): Modulationsring am Regler (Bogen in LFO-/EG-Farbe vom Reglerwert um den Betrag,
  `widgets/knob.rs`), Inspektor per Langdruck → "Modulate…" (`src/panels/modulation.rs`: Quelle Off/LFO/EG, Betrag
  bipolar, Fertig), und DRIVE-Regler auf der FILTER-Seite (CSTM `FILTER DRIVE`, Byte 96, Einstellung 16400; Verstärkung
  vor dem Filter, Ausgang gibt die Wurzel zurück). Der Ring zeigt die Reichweite, bewegt sich noch nicht live mit der
  Modulation (Live-Punkt wäre der nächste Schritt).
- Noch offen: Live-Punkt am Ring, EG-Quelle pro Stimme, mehr Quellen/Ziele, Charakter-Modi des Filters, P-Locks.

## 3e. Parameter-Locks, Stufe 1 (gebaut, Tests grün)

- Modell: `Lock { tick, key, target, value }` je Noten-Clip (`ClipFileState.locks`), Datei: Honeydew-Chunk `HDLk`
  (20-Byte-Einträge), nur geschrieben wenn benutzt; FL Studio Mobile überspringt ihn. Kollaborations-Liste kennt das Feld.
- Wiedergabe (`src/audio_engine/locks.rs`): je gelockter Note zwei geplante Parameter-Schreibungen: Wert beim Notenstart,
  gespeicherter Reglerwert beim Notenende. Nur Noten-Lanes, nur Instrument-Parameter (nicht Kanalregler, keine Drum-Hits).
  Grenzen: überlappende Noten geben den Parameter beim Ende der ersten zurück; ein automatisierter Parameter fällt am
  Notenende auf den gespeicherten Wert, nicht auf die Automation.
- Bedienung: Piano Roll, Grafikstreifen, dritter Eintrag **Lock**. Kopfzeile antippen → Parameterliste des Instruments
  (nur Regler, keine Menüs) und "Clear locks". Mit dem Finger über Noten malen setzt deren Lock-Wert; Noten ohne Lock zeigen
  eine dünne Linie auf dem Reglerwert. Ein Malzug ist ein Undo-Schritt.
- Noch offen: Locks folgen verschobenen/kopierten Noten (bisher bleibt der Lock an Zeit+Taste; Verschieben löst ihn von
  der Note), Drum-Editor/Step-Sequencer, Kanalregler, mehrere Parameter gleichzeitig sichtbar, Wert-Anzeige beim Malen.

## 3f. Feedback zur APK `45d52a7` und Umsetzung (Tag 3, gebaut, Tests grün)

- Filteranzeige: die senkrechte Hilfslinie läuft nicht mehr mit, die bewegte Kurve behält die Filterfarbe.
- DRIVE: deutlich stärker (Eingangsverstärkung bis 30x in einen weichen Clip vor dem Filter, Ausgang gibt Pegel zurück).
- MATRIX: Ziel-Menü hat "Off" als erstes Element (Dateiwerte der Test-APK verschieben sich um eins), Routen als 2x2-Raster mit
  großen AMOUNT-Reglern.
- Modulate-Panel im Dialog-Design (wie Fine tune): Quelle als drei solide Buttons, bipolarer Slider, Wert als Text; Tipp auf den
  Wert öffnet das Eingabefeld (Tastatur) für Prozent; "Done" schließt.
- Doppeltipp zum Zurücksetzen funktioniert auch mit zweitem Finger, während der erste eine Taste hält (`other_finger`).
- P-Locks: Locks werden vor den Noten geplant, damit der Wert springt statt zu gleiten; Header mit drei soliden Tabs
  (Parametername zentriert, zweiter Tipp auf den gewählten Tab öffnet die Liste), Lane im Look des Drum-Editors (Balken über
  der Note; mittig startende Balken für Pan/Pitch-artige Parameter).

## 3g. Später auf der Liste: Trance Delay aufwerten

Wunsch des Besitzers (Tag 3): Das Trance Delay ordentlich aufpimpen. Sehr wichtig: Timing in **3/16** (dotted eighth bzw.
drei Sechzehntel), außerdem eine übersichtlichere, modernere Oberfläche im eigenen Farbthema. Erst nach den P-Locks.

## 3h. Stand Ende Tag 3 (APK `9ef5e41`, Build erfolgreich, 76 Test-Gruppen grün)

Fertig und zum Testen auf dem Handy: Mod-Matrix (4 Routen, Off-Ziel), Modulationsring, Modulate-Dialog (Tipp auf Wert =
Tastatur), Filter-DRIVE, P-Locks im Piano Roll (3 Tabs, Drum-Editor-Look, Wert springt), Doppeltipp-Reset mit zweitem Finger,
**Wavetable-Oszillatoren** (`crates/audio/src/wavetable.rs`, `wavetable_factory.rs`; 12 Werks-Tabellen, POS A/B als Parameter 75/76,
3D-Vorschau `ridge` in `panels/inst/gm_synth.rs`, Matrix-Ziele Table Pos A/B).

Nächste Schritte (Reihenfolge nach Feedback des Besitzers):
1. Feedback zur Wavetable-APK abwarten (3D-Ansicht lesbar? Morphen glatt? 7 Regler auf OSC-Seite noch bedienbar?).
2. **WAV-Import für Wavetables** (Dateiauswahl wie 3x-Osc-Custom-Welle, Daten im Song speichern; vorher mit Fragen-Panel klären).
3. **P-Locks**: Locks folgen verschobenen/kopierten Noten; Drum-Editor/Step-Sequencer; Kanalregler; Wert-Anzeige beim Malen.
4. Modulationsring: Live-Punkt; EG-Quelle pro Stimme.
5. Filter-Charakter-Modi (nach DRIVE-Feedback).
6. **Trance Delay** aufwerten (3/16-Timing, moderne UI im Farbthema), siehe 3g.
7. Upstream-Check (`daijro/honeydew`) vor größeren Vorhaben, Änderungen gezielt übernehmen.

## 3i. Design im Stil moderner Wavetable-Synths (Tag 4, Schritt 1, gebaut, Tests grün, NOCH NICHT als APK)

Auftrag des Besitzers (Referenz: Pigments-Screenshot): die ganze App soll diesem Design folgen: dunkle Tastatur, neu gestaltete
Regler. Entscheidungen: Umfang "Theme + Regler + Keyboard", Regler-Look nur im neuen Theme (Theme-Schalter), Build erst nach dem
Wavetable-Test des Besitzers: dieser Stand ist nur lokal committet und nicht gepusht, bis der Besitzer es freigibt.

- Neues Built-in-Theme **Graphite** (Nr. 9): dunkles Graphit (`#16181B` bis `#30353B`), Cyan-Akzent `#2CB5E8`, dunkle Tastatur
  (weiß `#8A8F96`, schwarz `#1A1C1F`, gedrückte Tasten in der Kanalfarbe), `knobArc` an.
- Neuer Theme-Schalter `knobArc` (`ThemeOptions.knob_arc`, JSON `knobArc`, Standard aus): Regler als dünner Wertebogen um einen
  dunklen Körper mit hellem Zeiger statt des klassischen Rings (`ui/widgets/knob.rs`, `paint_arc_knob`). Bogen vom Ruhepunkt
  (unipolar: Anfang, bipolar: Mitte) zum Wert in der Kanal-/Akzentfarbe, Spur dunkel; der Modulationsring liegt außerhalb.
  Gilt für Regler, Strip-Regler und Graph-Regler (alle laufen durch `dial`). Eigene Themes können `"knobArc": true` setzen.
- Offen für die nächsten Design-Schritte (vom Besitzer noch nicht beauftragt): abgerundete Karten mit dünnem Rand, Titelzeilen in
  Großbuchstaben mit Akzent, Tabs mit farbiger Unterlinie, Wert unter dem Regler statt im Ring, Modulationsquellen-Leiste.

## 4. Offen (in der Reihenfolge, die ich empfehle; Besitzer hat die Wünsche genannt)

**A. (ERLEDIGT, siehe 3b) Anbindung „Instrument → Oberfläche" (Spektrum und LFO-Phase)** – ursprünglich Voraussetzung für zwei Wünsche:
 - *Spektrum-Analyser als Hintergrund der Filter-Anzeige, reagiert auf den Filter.* Der Analyzer-Effekt existiert; Instrumente
   liefern bisher keine Analyse (`AudioRenderer::analysis(index)` ist für Instrumente leer; `crates/audio/src/device.rs`
   `Visual`-Seqlock-Slab pro Meter, Größe je Modultyp; `src/audio_engine/mod.rs` `analysis()`; App: `panels/fx/mod.rs`
   `live_analysis`, `post_feed`). Plan: `Instrument::analysis()` (Standard leer), GM Synth rechnet ein grobes Spektrum seiner
   Ausgabe (allokationsfrei) und die LFO-Phase, Engine/Meter-Slab für Instrumente erweitern.
 - *Laufender Punkt auf der LFO-Kurve synchron zur Modulation* (braucht die LFO-Phase).
**B. (TEILS ERLEDIGT, siehe 3b) LFO-Erweiterung:** Sample & Hold, Sync-Modi (1/16 …), **„bei jeder Note ein neuer Zufallswert"** mit frei einstellbaren
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

## 4b. Plan: Parameter-Locks (Elektron-Stil) im Piano-Roll-Grafikstreifen

Stand: nur Plan, noch nichts gebaut. Ausgangslage im Code: `src/ui/graph_lane.rs` zeigt unter dem Raster eine Liste
(`Vel`/`Pan`, `Prop`) und malt eine Eigenschaft der Noten. `Event::Note` (`src/song/lane.rs`) hat nur `velocity`, `pan`,
`slide`. Automationsziele (`AutomationTarget{instance_id,param}`) gibt es schon, aber nur als eigene Lane mit Kurve.

Idee: Ein P-Lock ist ein Wert für einen Modul-Parameter, der nur für eine Note (Schritt) gilt.
1. Bedienung: Im Streifen ein "+"-Eintrag unten in der Liste. Tippen öffnet die Parameterliste des Kanal-Instruments
   (Name aus `src/params`). Der gewählte Parameter wird ein neuer Listeneintrag (Farbe wie sein Modul-Akzent).
2. Wert setzen: wie bei Vel - Balken pro Note mit dem Finger malen oder ziehen. Noten ohne Lock zeigen den
   Basiswert des Reglers als dünne Linie; Lock = gefüllter Balken.
3. Speicherung: neue Liste `locks: Vec<(instance_id, param, value)>` je Note. Kompatibilität: `.flm` kennt das nicht -
   Variante A (empfohlen): in einem eigenen Honeydew-Chunk speichern (unbekannte Chunks bleiben erhalten), Variante B:
   unsichtbare Automation-Lane je Parameter mit Stufen. B ist file-kompatibel, aber unscharf bei überlappenden Noten.
4. Wiedergabe: Der Sequencer setzt beim Notenbeginn den Parameter über die bestehende Parameter-Queue
   (`audio_link`), bei Notenende zurück auf den Basiswert. Erst nur für Parameter, die live änderbar sind (Cutoff usw.).
5. Schritte: (a) Datenmodell + Test, (b) Wiedergabe + Test, (c) UI-Liste mit "+", (d) Malen/Ziehen, (e) Undo über
   `SongEdit::Final`. Jeder Schritt ein Commit; UI-Prototyp (c) zuerst mit dem Handy prüfen.
Erkenntnisse nach Code-Lesen (Tag 3), Entscheidung für den Bau:
- Automations-Lanes (Variante B) liegen als eigene Lanes mit eigenen Clips im Song; Noten und Lock-Werte würden beim
  Verschieben/Kopieren/Loopen von Noten auseinanderlaufen. Das wäre eine Fehlerquelle über die Zeit.
- Besser (Variante A, empfohlen): Locks hängen **an der Note im selben Clip**: `Clip.locks: Vec<Lock>` mit
  `Lock { note_pos, key, target: AutomationTarget, value }`; beim Verschieben/Löschen/Kopieren von Noten werden die Locks
  mitgeführt (ein einziger Ort: die Noten-Bearbeitungsfunktionen). Datei: eigener Honeydew-Chunk im CLIP (ungewohnte
  Chunks bleiben erhalten). Wiedergabe: beim Notenstart setzt der Sequencer den Parameter über denselben Weg wie
  Automation (`set_parameter` im Engine, `src/audio_engine/mod.rs`), nach Notenende zurück auf den Basiswert (oder wie bei
  Drambo bis zum nächsten Trigger halten, als Option).
- Reihenfolge: (1) Datenmodell + Test, (2) Datei-Chunk lesen/schreiben + Test, (3) Wiedergabe + Test, (4) Grafikstreifen-UI
  mit "+" und Malen. Jeder Schritt ein Commit; erst Schritt 4 braucht dein Feedback am Handy.

Offene Frage an den Besitzer (mit obiger Empfehlung A beantwortet, falls er nichts anderes sagt): nur Piano Roll oder auch Drum-Editor/Step-Sequencer.

> Entscheidung Tag 3: FL-Mobile-Kompatibilität nur noch beim Lesen; Schreiben darf neue Felder/Parameter nutzen.

## 4c. Plan: Wavetable-Oszillatoren (vorgezogen, Entscheidungen des Besitzers von Tag 3)

Entscheidungen: eine gemeinsame Auswahlliste (29 Shapes + Wavetables) pro Oszillator; die Position als echte Parameter
(Pos A, Pos B: Automation, Mod-Matrix, LFO/EG, P-Locks gehen darauf); erster Build mit WAV-Import; alles Bestehende (FM,
Ring, Sync, Choir, Filter) bleibt.

Ausgangslage: Ein Shape ist heute ein `Wavetable` aus 512 Tabellen (je Oberwellenzahl) zu 1024 Punkten, gelesen ohne
Interpolation mit Dither (`gm_waveshapes.rs`). Das bleibt für die 29 Shapes bit-genau erhalten.

Technik (neu, `crates/audio/src/wavetable.rs`):
- `FrameTable`: N Frames (bis 256) zu je 2048 Punkten, je Frame per FFT in Oberwellen zerlegt und in ~10 Oktav-Stufen
  bandbegrenzt gespeichert (kein Aliasing, wenige MB). Lesen: linear zwischen Punkten, Position blendet zwischen zwei
  Frames (flüssig, ohne Sprünge).
- Quelle pro Oszillator: `Shape(slot)` oder `Table(index)`; FM/Ring/Sync lesen über dieselbe Lese-Funktion.
- Werks-Tabellen (rechnerisch erzeugt, eigene Entwürfe): Morphs (Sinus-Saw-Square), PWM, Formant/Vokale, FM-Sweep,
  Harmonik-Sweep, Digital u. a.
- WAV-Import: Datei in Frames zu 2048 Punkten teilen (Serum-Konvention), auf Pegel normieren; Pfad/Daten wie bei der
  3x-Osc-Custom-Welle.
- UI: 3D-Vorschau (Frames als hintereinander gestaffelte Linien in Schrägansicht, aktuelle Position hervorgehoben, Ziehen
  setzt die Position), Position als Regler, Auswahlliste mit Tabellen unter den Shapes.
- Schritte (je ein Commit): (1) FrameTable + Lesen + Tests, (2) Werks-Tabellen, (3) Einbau in GMSynth (Quelle + Pos-Parameter,
  Datei/CSTM), (4) UI mit 3D-Vorschau, (5) WAV-Import, (6) Matrix-Ziele Pos A/B.

## 4d. Feature-Liste des Besitzers (Tag 4), geordnet und geplant (nur Planung, noch nichts gebaut)

Quelle: Liste des Besitzers vom Morgen von Tag 4. Zuerst der Abgleich mit dem, was schon im Code steht, dann die
Themen-Gruppen mit Aufwand (S klein, M mittel, L groß), Abhängigkeiten, Recherche und der empfohlene Ablauf.

### Abgleich der als erledigt markierten Punkte ([X] in der Liste)

| Punkt | Stand im Code | Offen davon |
|---|---|---|
| GM Synth Wavetable (zwei Tabellen, Pitch, Morph, FM) | Gebaut (Tag 3): 12 Werks-Tabellen, POS A/B, 3D-Vorschau, FM/Ring/Sync bleiben. Noch nicht auf dem Handy getestet. | **Warp-Modi** (siehe A1), WAV-Import (A2) |
| GM Synth Envelopes frei mappbar | Matrix mit LFO/EG als Quelle, bis 4 Routen, Ring am Regler, Inspektor. | Mehr als ein EG (pro Stimme), mehr Quellen (Velocity, Key, Zufall) |
| LFO-Upgrades (Random, S&H, Grafik, freies Mapping) | S&H, Note-Random, Tempo-Sync, Grafik, freie Ziele über die Matrix: gebaut. | Mehr LFOs (2+), freie Hz-Rate, LFO je Stimme |
| Drum-Sequencer-Automation (Zeilen, Pro-Schritt) | **Nur teilweise:** P-Locks gibt es im Piano Roll, nicht im Drum-/Step-Editor. | P-Locks im Drum-Editor und Step-Sequencer (A3/B), Rückfrage an den Besitzer: war mit [X] "geplant" oder "aus dem Original" gemeint? |

### Gruppen und Reihenfolge

**A. Klang und Synth** (Aufbau auf der frischen Wavetable-Engine)
- A1 *Warp-Modi für Wavetables* (M). Recherche: Serum bietet Sync (1/2 und voll, auch "Window"), Bend ±, Asym ±, PWM,
  Flip, Mirror, Quantize, FM, AM, RM. Plan: pro Oszillator ein Warp-Menü plus Warp-Regler (Parameter, damit mappbar), gerechnet
  beim Lesen der Phase (Phasenverzerrung ist billig und aliasarm, Quantize/Mirror brauchen Bandbegrenzung der Tabelle).
  Die Warp-Wirkung zeigt die 3D-Vorschau mit. Vorher mit dem Besitzer klären, welche 6 bis 8 Modi zuerst.
- A2 *Single-Cycle- und Wavetable-Import per WAV* (M, schon geplant): Datei in Frames zu 2048 Punkten, Einzelzyklus = 1 Frame.
  Speicherung der Daten im Song und Dateiauswahl wie bei der 3x-Osc-Custom-Welle. Vorher Fragen-Panel.
- A3 *Modulator-Modul*: zusätzliche LFO-Formen, Grafik für LFO und MSEG (mehrstufige Hüllkurve) (L). Baut auf der Anzeige-
  Technik der GM-Synth-LFO-Seite auf; MSEG braucht einen Punkte-Editor (Vorlage: Automations-Editor `src/ui/automation`).
- A4 *Mehr Hüllkurven/LFOs im GM Synth* (S bis M): Matrix-Quellen erweitern (EG 2, LFO 2, Velocity, Key).

**B. Automation 2.0 und P-Locks** (ein zusammenhängendes Großthema, das Fundament für P-Locks "für alle Parameter")
Reihenfolge innerhalb, weil jeder Schritt auf dem vorigen aufbaut:
1. B1 *Gemeinsame Skalierung und Einrasten* (Zeit und Wert) zwischen Automations-Lane und Projekt-Zeitleiste (M). Zuerst
   die Geometrie vereinheitlichen (ein Zeitachsen-Objekt für Playlist und Editoren; `ui/automation/geom`, `ui/timeline`).
2. B2 *Lane-Oberfläche wie bei Ableton* (L): klare Parameterbeschriftung an der Lane, direktes Bearbeiten in der Zeitleiste,
   Lanes mit dem Song mitskalieren. Plan: Lane-Kopf mit Name "Parameter : Modul", Farbe des Moduls, Auswahl per Tipp.
3. B3 *Automation direkt im Piano Roll zeichnen* (L): derselbe Zeichnen-Editor als Streifen unter den Noten (Grafikstreifen
   existiert; die Lock-Lane ist die Vorstufe). Hier laufen P-Locks (pro Note) und Kurven (pro Zeit) zusammen.
4. B4 *S-Pen-Unterstützung* (M): flüssiges Zeichnen von Kurven plus Stepped-Modus. Recherche: Android liefert Stylus als
   `TOOL_TYPE_STYLUS` mit Druck, Neigung und vielen Zwischenpunkten pro Frame (`getHistorical…`), dazu gibt es Bibliotheken
   für niedrige Latenz und Bewegungsvorhersage. Risiko: Das vendorte `winit` reicht den Stift nur als `Touch` mit optionalem
   `force` durch, die Zwischenpunkte kommen nicht an. Vermutlich braucht es einen kleinen Java-Teil in `HoneydewActivity`, der
   Stylus-Punkte per JNI (Muster wie `onMic`/`onHeadphones` in `src/android.rs`) an die Automation liefert. Erst prüfen, ob
   `force` und die Touch-Rate reichen (billig), sonst die JNI-Variante.
5. P-Lock-Reste (siehe 3h): Locks folgen Noten, Drum-/Step-Editor, Kanalregler.

**C. Sequencing, Routing, Instrumente**
- C1 *MIDI-Note steuert eigenen Synth im selben Rack* und *Effekte nur einem Synth zuweisen* (L, hängt zusammen). Heute hat
  jeder Kanal ein Rack mit Instrumenten und Effekten, die alle denselben Kanal hören; `midi_sends` verbindet Kanäle. Plan:
  Rack in "Stimmen/Zweige" gliedern (Instrument + seine Effektkette), jeder Zweig mit eigener Note-Quelle (Notenspur, Drum-Pad,
  Arp). Das ist eine Änderung am Datenmodell (`Rack`) und an `audio_engine`, also gründlich testen. Vorher Fragen-Panel mit
  Skizze, weil es die Bedienung des Racks stark verändert.
- C2 *Drum-Rack-Sequenzen an Synths per MIDI-Sends* (M): auf C1 aufbauen oder vorher als Zwischenlösung die vorhandenen
  Sends für Drum-Pads erlauben.
- C3 *Arpeggiator-Update* (L): eigene Muster pro Schritt (Pitch, Velocity, Länge, Wahrscheinlichkeit, Transposition),
  Parameter-Steuerung, Zwei-Arp-Zusammenspiel. Recherche: Ableton hat dafür das Step Arp (Sequencers-Pack, Max for Live) mit sechs
  Spuren (Velocity, Länge, Chance, Transposition …), jede mit eigener Schrittzahl und Richtung; "Snake"-artige Ideen stammen
  von LFO-Phasen-Sequenzern (z. B. LF-ARP); Dual-Arp-Geräte gibt es nach dem Vorbild von Reason. Plan: neuer Arp-Modus
  "Pattern" mit Spuren und eigener Länge je Spur (polyrhythmisch), Anzeige wie die Drum-Editor-Lane. Vorlage im Code:
  `crates/audio/src/sequencer.rs` (Arpeggiator) und `src/panels/fx/arpeggiator.rs`.
- C4 *Slicer: manuelle Regler für Länge, Start und Ende der Slices* (M): Die Slicer-Karte hat heute Sensitivity, Pitch,
  Attack, Decay. Plan: Wellenform-Ansicht mit ziehbaren Slice-Markern (Vorlage: Audio-Editor `ui/audio_editor`) und Start/Ende-Regler pro Slice.

**D. Arbeitsfluss und Oberfläche**
- D1 *Clips manuell einfärben* (S): Kanal hat `hue`; ein Clip bekommt optional eigene Farbe (neues Feld im Clip, eigener
  Honeydew-Chunk, Menü "Farbe" im Clip-Menü).
- D2 *Marker-Spur mit Text* (S bis M): Es gibt bereits `Section` (benannte Bereiche, `src/song/section.rs`). Plan: Marker als
  Punkt-Markierungen mit Text auf einer eigenen Spur über der Zeitleiste, Sections bleiben Bereiche.
- D3 *Sample-Browser Drag & Drop* auf die Zeitleiste und in Drum-Sequencer-Lanes (M). Ansatz da: der Modul-Browser kennt
  schon "Lang drücken, dann ziehen" (`ui/file_drag`, `modules_browser.rs`). Ziel-Erkennung in Playlist und Drum-Lanes ergänzen.
- D4 *EQ: mehr Bänder und Analyser direkt im EQ* (M): Der Equalizer hat 4 Bänder; mehr Bänder heißt mehr Parameter (Honeydew-
  eigene, wie beim GM Synth), die Anzeige zeigt dann das Spektrum der Wavetable-/Filter-Anzeige-Technik (`SpectrumProbe`
  im Audio-Crate) direkt hinter der Kurve.
- D5 *Rack-Scrollleiste mit Klick und Hervorhebung* (M): Die Leiste schnappt je Modul ein (wie die App-Schublade bei Android), das
  Modul skaliert kurz leicht hoch. Braucht Haptik (D6) für den "Klick".
- D6 *Haptik* (M, danach überall einsetzbar): Recherche: Android bietet `VibrationEffect` mit vordefinierten Stärken (CLICK,
  TICK, LOW_TICK, jeweils skalierbar, älter als "Composition-Primitives" mit Skalierung und Verzögerung). Die Geräte-
  Unterstützung ist unterschiedlich (`Vibrator.arePrimitivesSupported` prüfen). Hinweis aus der Doku: viele Ticks hintereinander
  addieren sich in der Wahrnehmung, also kalibrieren. Plan: ein kleines Haptik-Modul (Rust → Java über JNI wie `src/android.rs`)
  mit wenigen benannten Ereignissen (Regler-Raster, Tab-Wechsel, Menü-Auswahl, Rastpunkt der Scrollleiste, Drücken) statt
  einer langen Vibration; jedes Ereignis mit eigener Stärke/Länge und zeitlich an die Animation gekoppelt; Schalter und
  Stärke in den Einstellungen. Vorher das Gerät des Besitzers erfragen (Samsung-Haptik-Motor kann feiner sein).
- D7 *MIDI-Controller-Unterstützung* (S bis M): Im Code ist MIDI-Eingabe/-Ausgabe schon da (`src/midi`, MIDI-CC-Links pro
  Kanal). Unklar, was fehlt: **Rückfrage** (Fragen-Panel): MIDI-Learn auf jeden Regler, Controller-Presets, Pitchbend/Aftertouch,
  USB-OTG am Handy, oder Bluetooth-MIDI?

### Empfohlener Ablauf (zur Bestätigung durch den Besitzer)
1. Tag-3-APK testen (Feedback, Fehler beheben), dazu WAV-Import (A2) und Lock-Folge-Verhalten (B5).
2. **Automation 2.0** (B1 bis B4): größter Hebel, trägt P-Locks, Wavetable-Position, Arp-Muster und Modulator.
3. **Routing im Rack** (C1, C2) mit Skizzen-Abstimmung; danach Arp (C3) und Slicer (C4).
4. **Klang-Ausbau** (A1 Warp, A3 Modulator/MSEG, A4) und Trance Delay (3g).
5. **Arbeitsfluss** (D1 bis D7), in kleinen Paketen zwischen den großen Themen eingestreut (D1, D2 sind schnell fertig).
6. Haptik (D6) früh anfangen, weil sie viele andere Punkte (Scrollleiste, Regler) verbessert.

### Quellen der Recherche
- Android-Stift: [Stylus input](https://developer.android.com/develop/ui/views/touch-and-input/stylus-input/advanced-stylus-features)
  (Druck, `TOOL_TYPE_STYLUS`, Zwischenpunkte `getHistorical…`, Low-Latency- und Motion-Prediction-Bibliotheken).
- Android-Haptik: [Custom haptic effects](https://developer.android.com/develop/ui/views/haptics/custom-haptic-effects) und
  [Constants and primitives](https://source.android.com/docs/core/interaction/haptics/haptics-constants-primitives).
- Warp-Modi: [Serum-Warp-Menü (Kurs)](https://phoenix.creativelive.com/class/xfer-serum-synthesizer-sound-design-music-production-tomas-george/lessons/warp-menu),
  [Wavetable Wave & Warp Effects (Minimal Audio)](https://support.minimal.audio/articles/29407564499739-wavetable-wave-and-warp-effects).
  Vital: dazu keine verlässliche Quelle gefunden, erst beim Bauen genauer ansehen.
- Arpeggiator: [Step Arp, Sequencers-Pack](https://www.ableton.com/en/packs/sequencers/),
  [DualArpPlus (Max for Live)](https://maxforlive.com/library/device/10150/dualarpplus).
  Ein Gerät, das "Snake" und Dual-Arp zugleich bietet, wurde nicht gefunden; "MDD Snake" bleibt für den Besitzer zu erklären.

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
