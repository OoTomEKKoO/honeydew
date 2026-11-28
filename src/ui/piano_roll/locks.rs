//! The piano roll's `Lock` item of the graph lane (Honeydew's own, [`crate::song::Lock`]): the pure parts. Which
//! parameters a channel's instruments offer, and how a painting finger sets the locks of the notes under it.
//! The drawing and the touch handling are `PianoRoll::lock_lane`.

use super::notes::note;
use crate::params::{ParamKind, module_spec};
use crate::song::{AutomationTarget, Channel, Clip, Event, Lock, PPQ};
use crate::ui::rack::is_instrument;

/// The parameters of `channel`'s instruments a note can lock, named as the automation names them: knobs and sliders
/// with a name (menus and switches would jump, and unnamed ones are internal), in rack order. With more than one
/// instrument in the rack each name carries its instrument's position.
pub fn lockable(channel: &Channel) -> Vec<(AutomationTarget, String)> {
    let instruments: Vec<_> = (channel.rack.modules.iter())
        .filter(|m| is_instrument(m.type_id))
        .collect();
    let mut out = Vec::new();
    for (n, module) in instruments.iter().enumerate() {
        let Some(spec) = module_spec(module.type_id) else {
            continue;
        };
        for (index, param) in spec.params.iter().enumerate() {
            let smooth = matches!(param.kind, ParamKind::Continuous { .. });
            if param.name.is_empty() || !smooth || index >= module.params.len() {
                continue;
            }
            let name = if instruments.len() > 1 {
                format!("{} ({})", param.name, n + 1)
            } else {
                param.name.to_owned()
            };
            out.push((
                AutomationTarget {
                    instance_id: module.instance_id,
                    param: index as i32,
                },
                name,
            ));
        }
    }
    out
}

/// `target`'s saved value on `channel` (what a note without a lock plays), if it has one.
pub fn base_value(channel: &Channel, target: AutomationTarget) -> Option<f32> {
    let module = (channel.rack.modules.iter()).find(|m| m.instance_id == target.instance_id)?;
    module
        .params
        .get(usize::try_from(target.param).ok()?)
        .copied()
}

/// Whether `target` rests in the middle (a knob whose default is 0.5: Pan, a pitch): its lock bars grow from the centre
/// band as the drum editor's Pan and Pitch do.
pub fn is_bipolar(channel: &Channel, target: AutomationTarget) -> bool {
    let module = (channel.rack.modules.iter()).find(|m| m.instance_id == target.instance_id);
    let spec = module.and_then(|m| module_spec(m.type_id));
    let param = spec.and_then(|s| s.params.get(usize::try_from(target.param).ok()?));
    param.is_some_and(|p| p.default == 0.5)
}

/// Locks `target` to `value` (0…1) on every eligible note whose extent [pos, pos + len) holds `tick`: all the notes, or
/// only the selected ones while any is selected (`selected` is parallel to the clip's events, as the other graph lane
/// items'). Returns whether a lock changed.
pub fn paint_locks(
    clip: &mut Clip,
    selected: &[bool],
    target: AutomationTarget,
    tick: f64,
    value: f32,
) -> bool {
    let any = selected.contains(&true);
    let mut painted = Vec::new();
    for (i, e) in clip.events.iter().enumerate() {
        let Some((pos, len, key)) = note(e) else {
            continue;
        };
        let eligible = !any || selected.get(i) == Some(&true);
        if eligible && pos <= tick && tick < pos + len {
            painted.push((pos as u32, key));
        }
    }
    let mut changed = false;
    for (pos, key) in painted {
        let now = clip.lock(pos, key, target).map(|l| l.value);
        if now != Some(value.clamp(0.0, 1.0)) {
            clip.set_lock(pos, key, target, value);
            changed = true;
        }
    }
    changed
}

/// Removes every lock of `target` in `clip`; returns whether there was one.
pub fn clear_locks(clip: &mut Clip, target: AutomationTarget) -> bool {
    let before = clip.file.locks.len();
    clip.file.locks.retain(|l| l.target != target);
    clip.file.locks.len() != before
}

/// The locks after the roll's own edit turned the clip's notes `old` into `new`: each lock goes with its note.
///
/// The roll's edits (a drag, a nudge, a transpose, a quantize, a humanize, a stretch, a delete) are many code paths that all
/// end in a new list of events, so the notes are matched here, once, from the two lists:
/// 1. A note that is still there with the same start and key keeps its place and its locks.
/// 2. The notes left over on each side are paired by nearness (start and key; a key counts as much as a bar), the closest
///    pairs first, and a lock moves to the new place of its note. With very many left over (a whole long clip moved at
///    once) they are paired in order, which a move keeps.
/// 3. A note with no partner was deleted: its locks go. A new note with no partner (a drawn, pasted or duplicated one)
///    starts without locks.
///
/// Returns `None` when no lock changes.
pub fn reconcile(old: &[Event], new: &[Event], locks: &[Lock]) -> Option<Vec<Lock>> {
    use std::collections::HashMap;
    if locks.is_empty() {
        return None;
    }
    let notes = |events: &[Event]| -> Vec<(u32, u8)> {
        events
            .iter()
            .filter_map(|e| match *e {
                Event::Note { pos, key, .. } => Some((pos, key)),
                _ => None,
            })
            .collect()
    };
    let (old, new) = (notes(old), notes(new));
    // Step 1: what stayed, as a count per (start, key).
    let mut left: HashMap<(u32, u8), isize> = HashMap::new();
    for &n in &old {
        *left.entry(n).or_default() += 1;
    }
    let mut fresh = Vec::new();
    for &n in &new {
        match left.get_mut(&n) {
            Some(count) if *count > 0 => *count -= 1,
            _ => fresh.push(n),
        }
    }
    let mut gone: Vec<(u32, u8)> = left
        .into_iter()
        .flat_map(|(n, count)| std::iter::repeat_n(n, count.max(0) as usize))
        .collect();
    if gone.is_empty() && fresh.is_empty() {
        return None;
    }
    // Step 2: pair the rest.
    gone.sort_unstable();
    fresh.sort_unstable();
    let cost = |a: (u32, u8), b: (u32, u8)| {
        u64::from(a.0.abs_diff(b.0)) + u64::from(a.1.abs_diff(b.1)) * u64::from(PPQ) * 4
    };
    let mut moved: HashMap<(u32, u8), (u32, u8)> = HashMap::new();
    let mut paired_old = vec![false; gone.len()];
    if gone.len().saturating_mul(fresh.len()) > 250_000 {
        for (i, (&a, &b)) in gone.iter().zip(&fresh).enumerate() {
            moved.insert(a, b);
            paired_old[i] = true;
        }
    } else {
        let mut taken = vec![false; fresh.len()];
        // 2a: a move or a transpose shifts a whole selection by one offset, which nearness alone can mistake when the shift
        // is larger than the notes' spacing. The offset that most of the leftovers share pairs them first.
        let delta = |a: (u32, u8), b: (u32, u8)| {
            (
                i64::from(b.0) - i64::from(a.0),
                i64::from(b.1) - i64::from(a.1),
            )
        };
        let mut votes: HashMap<(i64, i64), usize> = HashMap::new();
        for &a in &gone {
            for &b in &fresh {
                *votes.entry(delta(a, b)).or_default() += 1;
            }
        }
        let best = votes
            .iter()
            .max_by_key(|(d, n)| (**n, std::cmp::Reverse(d.0.abs() + d.1.abs())))
            .map(|(d, n)| (*d, *n));
        if let Some(((dt, dk), _)) = best.filter(|&(_, n)| n >= 2) {
            for (i, &a) in gone.iter().enumerate() {
                let want = (i64::from(a.0) + dt, i64::from(a.1) + dk);
                let hit = fresh
                    .iter()
                    .enumerate()
                    .find(|&(j, &b)| !taken[j] && (i64::from(b.0), i64::from(b.1)) == want);
                if let Some((j, &b)) = hit {
                    (paired_old[i], taken[j]) = (true, true);
                    moved.insert(a, b);
                }
            }
        }
        // 2b: the rest by nearness, the closest pairs first.
        let mut pairs: Vec<(u64, usize, usize)> = (0..gone.len())
            .filter(|&i| !paired_old[i])
            .flat_map(|i| (0..fresh.len()).filter(|&j| !taken[j]).map(move |j| (i, j)))
            .map(|(i, j)| (cost(gone[i], fresh[j]), i, j))
            .collect();
        pairs.sort_unstable();
        for (_, i, j) in pairs {
            if !paired_old[i] && !taken[j] {
                (paired_old[i], taken[j]) = (true, true);
                moved.insert(gone[i], fresh[j]);
            }
        }
    }
    // Step 3: what no partner was found for is deleted. (A start and key that also has a surviving note keeps its locks:
    // the lock list cannot tell the two apart.)
    let deleted: Vec<(u32, u8)> = gone
        .iter()
        .zip(&paired_old)
        .filter(|(_, paired)| !**paired)
        .map(|(n, _)| *n)
        .collect();
    let stays = |n: (u32, u8)| new.contains(&n);
    let (mut kept, mut followed): (Vec<Lock>, Vec<Lock>) = (Vec::new(), Vec::new());
    for lock in locks {
        let at = (lock.tick, lock.key);
        if let Some(&(tick, key)) = moved.get(&at) {
            followed.push(Lock { tick, key, ..*lock });
            // A twin of the note left behind (same start and key still in the clip) keeps its own.
            if stays(at) {
                kept.push(*lock);
            }
        } else if !(deleted.contains(&at) && !stays(at)) {
            kept.push(*lock);
        }
    }
    // One lock per note and parameter: when two land on the same one, the moved note's wins.
    let mut out = kept;
    out.extend(followed);
    out.sort_by_key(Lock::order);
    out.reverse();
    out.dedup_by_key(|l| l.order());
    out.reverse();
    (out != locks).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::{ChannelKind, Event, Song, SynthKind};

    fn clip_with_notes() -> Clip {
        let mut lane = crate::song::Lane::new(crate::song::LaneKind::Notes);
        let clip = lane.add_clip(0, 4.0);
        for (pos, key) in [(0, 60), (128, 62), (256, 64)] {
            clip.events.push(Event::Note {
                pos,
                len_beats: 1.0,
                key,
                velocity: 200,
                pan: 127,
                slide: false,
            });
        }
        clip.clone()
    }

    const T: AutomationTarget = AutomationTarget {
        instance_id: 3,
        param: 21,
    };

    fn note_at(pos: u32, key: u8) -> Event {
        Event::Note {
            pos,
            len_beats: 1.0,
            key,
            velocity: 200,
            pan: 127,
            slide: false,
        }
    }

    fn lock(tick: u32, key: u8, value: f32) -> Lock {
        Lock {
            tick,
            key,
            target: T,
            value,
        }
    }

    #[test]
    fn a_lock_goes_with_its_note_when_the_note_moves_transposes_or_quantizes() {
        let old = [note_at(0, 60), note_at(128, 62), note_at(256, 64)];
        let locks = [lock(0, 60, 0.1), lock(128, 62, 0.2), lock(256, 64, 0.3)];
        // The middle note moves later.
        let new = [note_at(0, 60), note_at(200, 62), note_at(256, 64)];
        let got = reconcile(&old, &new, &locks).unwrap();
        assert_eq!(
            got,
            [lock(0, 60, 0.1), lock(200, 62, 0.2), lock(256, 64, 0.3)]
        );
        // All three go up a fifth: the locks follow every key.
        let new = [note_at(0, 67), note_at(128, 69), note_at(256, 71)];
        let got = reconcile(&old, &new, &locks).unwrap();
        assert_eq!(
            got,
            [lock(0, 67, 0.1), lock(128, 69, 0.2), lock(256, 71, 0.3)]
        );
        // A quantize moves two a little, in different directions.
        let old = [note_at(10, 60), note_at(120, 62)];
        let new = [note_at(0, 60), note_at(128, 62)];
        let got = reconcile(&old, &new, &[lock(10, 60, 0.4), lock(120, 62, 0.6)]).unwrap();
        assert_eq!(got, [lock(0, 60, 0.4), lock(128, 62, 0.6)]);
    }

    #[test]
    fn deleting_a_note_drops_its_locks_and_a_new_note_has_none() {
        let old = [note_at(0, 60), note_at(128, 62)];
        let locks = [lock(0, 60, 0.1), lock(128, 62, 0.2)];
        let got = reconcile(&old, &[note_at(0, 60)], &locks).unwrap();
        assert_eq!(got, [lock(0, 60, 0.1)]);
        // A drawn, pasted or duplicated note starts without locks, and nothing else changes.
        let new = [note_at(0, 60), note_at(128, 62), note_at(256, 64)];
        assert_eq!(reconcile(&old, &new, &locks), None);
        // Nothing changed, and no locks to follow.
        assert_eq!(reconcile(&old, &old, &locks), None);
        assert_eq!(reconcile(&old, &[], &[]), None);
    }

    #[test]
    fn a_moved_note_takes_the_place_of_a_locked_note_and_twins_keep_their_own() {
        // A lands on B's place: A's lock wins there (B is not there any more either).
        let old = [note_at(0, 60), note_at(128, 60)];
        let locks = [lock(0, 60, 0.1), lock(128, 60, 0.9)];
        let got = reconcile(&old, &[note_at(128, 60)], &locks).unwrap();
        assert_eq!(got.len(), 1);
        // Two notes on one start and key share its locks; moving one leaves the other its own and gives the mover a copy.
        let twins = [note_at(0, 60), note_at(0, 60)];
        let got = reconcile(
            &twins,
            &[note_at(0, 60), note_at(64, 60)],
            &[lock(0, 60, 0.5)],
        )
        .unwrap();
        assert_eq!(got, [lock(0, 60, 0.5), lock(64, 60, 0.5)]);
    }

    #[test]
    fn a_long_clip_moved_whole_keeps_every_lock_by_order() {
        // 700 notes moved by 10 ticks: more leftovers than the nearest-first pairing handles, so they pair in order.
        let old: Vec<Event> = (0..700)
            .map(|i| note_at(i * 128, 60 + (i % 12) as u8))
            .collect();
        let new: Vec<Event> = (0..700)
            .map(|i| note_at(i * 128 + 10, 60 + (i % 12) as u8))
            .collect();
        let locks: Vec<Lock> = (0..700)
            .map(|i| lock(i * 128, 60 + (i % 12) as u8, i as f32 / 700.0))
            .collect();
        let got = reconcile(&old, &new, &locks).unwrap();
        assert_eq!(got.len(), 700);
        for (i, l) in got.iter().enumerate() {
            assert_eq!(
                (l.tick, l.key),
                (i as u32 * 128 + 10, 60 + (i % 12) as u8),
                "lock {i}"
            );
            assert!((l.value - i as f32 / 700.0).abs() < 1e-6);
        }
    }

    #[test]
    fn painting_locks_the_notes_under_the_finger_or_only_the_selected_ones() {
        let mut clip = clip_with_notes();
        let none = [false; 3];
        assert!(paint_locks(&mut clip, &none, T, 130.0, 0.4));
        assert_eq!(clip.file.locks.len(), 1);
        assert_eq!(clip.lock(128, 62, T).unwrap().value, 0.4);
        assert!(
            !paint_locks(&mut clip, &none, T, 130.0, 0.4),
            "no change, no edit"
        );
        assert!(paint_locks(&mut clip, &none, T, 130.0, 2.0));
        assert_eq!(clip.lock(128, 62, T).unwrap().value, 1.0, "clamped");
        // With a selection only its notes take the value.
        let selected = [true, false, false];
        assert!(
            !paint_locks(&mut clip, &selected, T, 130.0, 0.1),
            "note 2 is not selected"
        );
        assert!(paint_locks(&mut clip, &selected, T, 10.0, 0.1));
        assert_eq!(clip.file.locks.len(), 2);
        // Off every note nothing happens.
        assert!(!paint_locks(&mut clip, &none, T, 5000.0, 0.5));
    }

    #[test]
    fn clearing_removes_only_that_parameters_locks() {
        let mut clip = clip_with_notes();
        let other = AutomationTarget { param: 22, ..T };
        paint_locks(&mut clip, &[false; 3], T, 130.0, 0.4);
        paint_locks(&mut clip, &[false; 3], other, 130.0, 0.6);
        assert!(clear_locks(&mut clip, T));
        assert_eq!(clip.file.locks.len(), 1);
        assert!(clip.lock(128, 62, other).is_some());
        assert!(!clear_locks(&mut clip, T));
    }

    #[test]
    fn a_gm_synth_offers_its_named_knobs_and_not_its_menus() {
        let mut song = Song::default();
        song.add_channel(ChannelKind::Synth(SynthKind::GmSynth));
        let channel = &song.channels[1];
        let id = channel.rack.modules[0].instance_id;
        let names: Vec<_> = lockable(channel)
            .into_iter()
            .map(|(t, n)| (t.param, t.instance_id, n))
            .collect();
        assert!(names.iter().all(|(_, i, _)| *i == id));
        let has = |name: &str| names.iter().any(|(_, _, n)| n == name);
        assert!(has("Cutoff") && has("Resonance") && has("Level"));
        assert!(
            !has("Filter Kind") && !has("LFO Shape"),
            "menus jump, so they are not offered"
        );
        assert!(base_value(channel, lockable(channel)[0].0).is_some());
        // Pan rests in the middle, Cutoff does not.
        let find = |name: &str| {
            lockable(channel)
                .into_iter()
                .find(|(_, n)| n == name)
                .unwrap()
                .0
        };
        assert!(is_bipolar(channel, find("Pan")));
        assert!(!is_bipolar(channel, find("Cutoff")));
    }
}
