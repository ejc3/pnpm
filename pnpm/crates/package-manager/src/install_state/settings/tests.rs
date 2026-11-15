use super::release_age_now_moved_back;

#[test]
fn release_age_now_drifts_only_when_it_moves_back() {
    let at = |value: &str| chrono::DateTime::parse_from_rfc3339(value).expect("valid").to_utc();
    let now = at("2026-03-01T00:00:00Z");
    let early = Some("2026-01-01T00:00:00Z");
    let late = Some("2026-02-01T00:00:00Z");
    let ahead = Some("2026-03-01T00:05:00Z");
    assert!(!release_age_now_moved_back(early, late, now), "moved forward");
    assert!(!release_age_now_moved_back(early, early, now), "unchanged");
    assert!(!release_age_now_moved_back(early, None, now), "removed while in the past");
    assert!(!release_age_now_moved_back(None, None, now), "never set");
    assert!(release_age_now_moved_back(late, early, now), "moved back");
    assert!(
        release_age_now_moved_back(None, early, now),
        "set where the record used its own clock",
    );
    assert!(release_age_now_moved_back(ahead, None, now), "removed while ahead of the clock");
}
