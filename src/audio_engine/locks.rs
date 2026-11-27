//! Parameter locks (Honeydew's own, [`crate::song::Lock`]): while a note plays, a parameter of its channel takes the
//! note's locked value, and the knob's own value comes back when the note ends.
//!
//! Playback's actions are prepared for the whole song ahead of time (`prepare`), so a lock is two scheduled parameter
//! writes: the locked value at the note's start frame and the module's own (saved) value at its end frame. The writes
//! go through the same path as automation's, so seeking into a note finds the right value too.
//!
//! Limits, kept on purpose to keep this simple and safe: only notes lanes (not drum hits yet), only a module's params
//! (not the channel strip), and the value that comes back is the module's saved one, so a lock on a param that is also
//! automated hands the param back to its saved value, not to the automation's current one. Notes that overlap hand the
//! param back when the first of them ends.

use super::EngineError;
use super::prepare::{Action, ScheduledAction, beat_frame};
use crate::audio_bridge::playing_clips;
use crate::song::{Event, LaneKind, PPQ, Song};
use honeydew_audio::instrument::InstrumentModule;

/// Adds the lock writes of `channel`'s notes lanes to `actions`: the ends first, then the starts, so that at one frame
/// (a note that ends where the next begins) the next note's lock is the last write and wins.
pub(super) fn actions(
    song: &Song,
    channel: usize,
    instruments: &[(i32, InstrumentModule)],
    end_beat: f64,
    sample_rate: f32,
    tempo_bpm: f32,
    actions: &mut Vec<ScheduledAction>,
) -> Result<(), EngineError> {
    let ch = &song.channels[channel];
    let end_tick = (end_beat * f64::from(PPQ)).ceil() as u64;
    let (mut starts, mut ends) = (Vec::new(), Vec::new());
    for lane in ch.lanes.iter().filter(|l| l.kind == LaneKind::Notes) {
        for clip in playing_clips(lane).filter(|c| !c.file.locks.is_empty()) {
            let clip_end = (f64::from(clip.start_tick) + clip.visible_ticks()) / f64::from(PPQ);
            for (tick, event) in clip.events_in_song_range(0, end_tick) {
                let Event::Note {
                    pos,
                    len_beats,
                    key,
                    slide: false,
                    ..
                } = *event
                else {
                    continue;
                };
                if !(len_beats > 0.0 && len_beats.is_finite()) {
                    continue;
                }
                let on = tick as f64 / f64::from(PPQ);
                let off = (on + len_beats).min(clip_end);
                if off <= on {
                    continue;
                }
                // The note's locks: the run of the sorted list with this tick and key.
                let locks = &clip.file.locks;
                let from = locks.partition_point(|l| (l.tick, l.key) < (pos, key));
                for lock in locks[from..]
                    .iter()
                    .take_while(|l| (l.tick, l.key) == (pos, key))
                {
                    let target = lock.target;
                    let Some(base) = base_value(ch, instruments, target.instance_id, target.param)
                    else {
                        continue;
                    };
                    let write = |frame: u64, value: f32| ScheduledAction {
                        frame,
                        channel,
                        action: Action::Param {
                            instance_id: target.instance_id,
                            index: target.param,
                            value,
                        },
                    };
                    starts.push(write(beat_frame(on, sample_rate, tempo_bpm)?, lock.value));
                    ends.push(write(beat_frame(off, sample_rate, tempo_bpm)?, base));
                }
            }
        }
    }
    actions.extend(ends);
    actions.extend(starts);
    Ok(())
}

/// The saved value of module `instance_id`'s param `index` when the module plays on this channel and has the param.
fn base_value(
    channel: &crate::song::Channel,
    instruments: &[(i32, InstrumentModule)],
    instance_id: i32,
    index: i32,
) -> Option<f32> {
    if instance_id <= 0 || !instruments.iter().any(|(id, _)| *id == instance_id) {
        return None;
    }
    let module = channel
        .rack
        .modules
        .iter()
        .find(|m| m.instance_id == instance_id)?;
    module.params.get(usize::try_from(index).ok()?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_cache::SampleCache;
    use crate::song::{AutomationTarget, ChannelKind, SynthKind};

    /// A song whose channel 1 is a GM Synth with one two-beat note clip (looping over four beats) holding a note at 0
    /// and one at beat 1, and the synth's instance id.
    fn song() -> (Song, i32) {
        let mut song = Song::default();
        song.add_channel(ChannelKind::Synth(SynthKind::GmSynth));
        let id = song.channels[1].rack.modules[0].instance_id;
        let clip = song.channels[1].lanes[0].add_clip(0, 2.0);
        clip.visible_len_beats = 4.0;
        for (pos, key) in [(0, 60), (PPQ, 64)] {
            clip.events.push(Event::Note {
                pos,
                len_beats: 1.0,
                key,
                velocity: 200,
                pan: 127,
                slide: false,
            });
        }
        (song, id)
    }

    fn run(song: &Song) -> Vec<(u64, i32, f32)> {
        let instruments = super::super::generators::build(
            &song.channels[1],
            48e3,
            120.0,
            64,
            &SampleCache::default(),
            Default::default(),
        );
        let mut out = Vec::new();
        actions(song, 1, &instruments, 4.0, 48e3, 120.0, &mut out).unwrap();
        out.iter()
            .filter_map(|a| match a.action {
                Action::Param { index, value, .. } => Some((a.frame, index, value)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_locked_note_sets_the_param_while_it_plays_and_gives_the_saved_value_back() {
        let (mut song, id) = song();
        let target = AutomationTarget {
            instance_id: id,
            param: 21,
        };
        assert!(run(&song).is_empty(), "no lock, no writes");
        let saved = song.channels[1].rack.modules[0].params[21];
        song.channels[1].lanes[0].clips[0].set_lock(0, 60, target, 0.2);
        // At 120 BPM and 48 kHz a beat is 24000 frames. The clip plays twice in four beats (loop of two).
        let beat = 24_000;
        let writes = run(&song);
        assert_eq!(
            writes,
            [
                (beat, 21, saved),
                (3 * beat, 21, saved),
                (0, 21, 0.2),
                (2 * beat, 21, 0.2)
            ],
            "ends first, then starts; the note at beat 1 has no lock"
        );
    }

    #[test]
    fn locks_that_cannot_apply_are_ignored() {
        let (mut song, id) = song();
        let clip = &mut song.channels[1].lanes[0].clips[0];
        // A param the module does not have, a module that is not there, the channel strip and a muted clip.
        clip.set_lock(
            0,
            60,
            AutomationTarget {
                instance_id: id,
                param: 9999,
            },
            0.5,
        );
        clip.set_lock(
            0,
            60,
            AutomationTarget {
                instance_id: id + 77,
                param: 21,
            },
            0.5,
        );
        clip.set_lock(
            0,
            60,
            AutomationTarget {
                instance_id: 0,
                param: 1,
            },
            0.5,
        );
        assert!(run(&song).is_empty());
        let clip = &mut song.channels[1].lanes[0].clips[0];
        clip.set_lock(
            0,
            60,
            AutomationTarget {
                instance_id: id,
                param: 21,
            },
            0.5,
        );
        assert_eq!(run(&song).len(), 4);
        song.channels[1].lanes[0].clips[0].muted = true;
        assert!(run(&song).is_empty(), "a muted clip plays no locks");
    }

    #[test]
    fn a_note_that_ends_where_the_next_begins_hands_the_param_to_the_next_lock() {
        let (mut song, id) = song();
        let target = AutomationTarget {
            instance_id: id,
            param: 21,
        };
        let clip = &mut song.channels[1].lanes[0].clips[0];
        clip.set_lock(0, 60, target, 0.2);
        clip.set_lock(PPQ, 64, target, 0.9);
        let writes = run(&song);
        let at_beat: Vec<_> = writes.iter().filter(|w| w.0 == 24_000).collect();
        // The first note's end (saved value) comes before the second note's start (0.9): the last write wins.
        assert_eq!(at_beat.len(), 2);
        assert_eq!(at_beat[1].2, 0.9);
    }
}
