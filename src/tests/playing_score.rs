use crate::{
    beatmap::{Beatmap, BeatmapMeta, HitObject, HitObjectType, Lane},
    state::playing::{Judgement, add_score, calculate_accuracy},
};

/// Builds a beatmap with the given number of notes.
fn beatmap_with_notes(count: u32) -> Beatmap {
    Beatmap {
        meta: BeatmapMeta {
            title: "test".to_string(),
            artist: "test".to_string(),
            mapper: "test".to_string(),
            level: 1.0,
            level_name: "test".to_string(),
        },
        bpm: 120,
        beats_per_bar: 4,
        hit_objects: (0..count)
            .map(|i| HitObject {
                time: i * 100,
                lane: Lane::Up,
                kind: HitObjectType::Chip,
            })
            .collect(),
        audio_path: String::new(),
    }
}

/// One of every judgement, used where a mixed set is needed.
fn judgements() -> Vec<(Judgement, u32)> {
    vec![
        (Judgement::Perfect(0), 0),
        (Judgement::Great(20), 100),
        (Judgement::Ok(40), 200),
        (Judgement::Bad(60), 300),
        (Judgement::Miss(-200), 400),
    ]
}

#[test]
fn score_all_perfect_reaches_maximum() {
    let notes = 10u32;
    let beatmap = beatmap_with_notes(notes);
    let mut score = 0;

    for i in 0..notes {
        add_score(&beatmap, &Judgement::Perfect(i as i32), &mut score);
    }

    assert_eq!(score, 1000000);
}

#[test]
fn score_mixed_judgements() {
    let notes = 4u32;
    let beatmap = beatmap_with_notes(notes);
    let max_per_note = 1000000 / notes;
    let mut score = 0;

    add_score(&beatmap, &Judgement::Perfect(0), &mut score);
    add_score(&beatmap, &Judgement::Great(20), &mut score);
    add_score(&beatmap, &Judgement::Ok(40), &mut score);
    add_score(&beatmap, &Judgement::Miss(-200), &mut score);

    let expected = max_per_note + (max_per_note / 4) * 3 + (max_per_note / 4) * 2 + 0;
    assert_eq!(score, expected);
}

#[test]
fn score_miss_adds_nothing() {
    let beatmap = beatmap_with_notes(3);
    let mut score = 0;
    add_score(&beatmap, &Judgement::Miss(-200), &mut score);
    assert_eq!(score, 0);
}

#[test]
fn accuracy_all_perfect_is_one() {
    let judgements: Vec<(Judgement, u32)> =
        (0..8).map(|i| (Judgement::Perfect(i), i as u32)).collect();
    assert_eq!(calculate_accuracy(&judgements), 1.0);
}

#[test]
fn accuracy_mixed_judgements() {
    // 1.0 + 0.75 + 0.5 + 0.25 + 0.0 = 2.5 over 5 notes = 0.5
    let judgements = judgements();
    assert_eq!(calculate_accuracy(&judgements), 2.5 / 5.0);
}

#[test]
fn accuracy_empty_is_one() {
    assert_eq!(calculate_accuracy(&[]), 1.0);
}

#[test]
fn accuracy_all_misses_is_zero() {
    let judgements: Vec<(Judgement, u32)> = (0..4)
        .map(|i| (Judgement::Miss(-(i as i32)), i as u32))
        .collect();
    assert_eq!(calculate_accuracy(&judgements), 0.0);
}
