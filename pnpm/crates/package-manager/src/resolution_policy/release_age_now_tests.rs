use super::PickPolicy;
use pnpm_config::Config;

#[test]
fn minimum_release_age_now_sets_the_resolution_cutoff() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z").expect("valid").to_utc();
    let config = Config {
        minimum_release_age: Some(24 * 60),
        minimum_release_age_now: Some(now),
        ..Config::new()
    };
    let policy = PickPolicy::from_config(&config).expect("derive pick policy");
    assert_eq!(policy.published_by, Some(now - chrono::Duration::days(1)));
}
