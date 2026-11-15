use super::{
    Config, EnvVar, EnvVarOs, GetCurrentDir, GetHomeDir, HostNoHome, LinkProbe, OsString, Path,
    PathBuf, assert_eq, config_from_workspace_yaml, fs, io, safe_host_var, tempdir,
};
use crate::{TrustPolicy, workspace_yaml::LoadWorkspaceYamlError};
use chrono::{DateTime, Utc};

fn time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value).expect("valid timestamp").with_timezone(&Utc)
}

fn load_error(yaml: &str) -> LoadWorkspaceYamlError {
    let tmp = tempdir().expect("workspace tempdir");
    fs::write(tmp.path().join("pnpm-workspace.yaml"), yaml).expect("write to pnpm-workspace.yaml");
    Config::new().current::<HostNoHome>(tmp.path()).expect_err("the value is invalid")
}

#[test]
fn reads_minimum_release_age_now_from_the_workspace_yaml() {
    let config = config_from_workspace_yaml("minimumReleaseAgeNow: '2026-01-02T03:04:05+02:00'\n");
    assert_eq!(config.minimum_release_age_now, Some(time("2026-01-02T01:04:05Z")));
    assert_eq!(config.release_age_now(), time("2026-01-02T01:04:05Z"));
}

#[test]
fn rejects_a_minimum_release_age_now_that_is_not_a_timestamp_or_is_in_the_future() {
    for (yaml, reason) in [
        ("minimumReleaseAgeNow: yesterday\n", "is not an RFC 3339 timestamp"),
        ("minimumReleaseAgeNow: '2999-01-01T00:00:00Z'\n", "is later than the current time"),
    ] {
        let error = load_error(yaml);
        assert!(
            matches!(&error, LoadWorkspaceYamlError::InvalidMinimumReleaseAgeNow { reason: got, .. } if *got == reason),
            "{yaml}: got {error:?}",
        );
    }
}

#[test]
fn minimum_release_age_now_env_var_wins_over_the_workspace_yaml() {
    struct HostWithReleaseAgeNowEnv;
    impl EnvVar for HostWithReleaseAgeNowEnv {
        fn var(name: &str) -> Option<String> {
            match name {
                "PNPM_CONFIG_MINIMUM_RELEASE_AGE_NOW" => Some("2026-03-01T00:00:00Z".to_owned()),
                _ => safe_host_var(name),
            }
        }
    }
    impl EnvVarOs for HostWithReleaseAgeNowEnv {
        fn var_os(_: &str) -> Option<OsString> {
            None
        }
    }
    impl GetHomeDir for HostWithReleaseAgeNowEnv {
        fn home_dir() -> Option<PathBuf> {
            None
        }
    }
    inert_link_probe!(HostWithReleaseAgeNowEnv);
    host_current_dir!(HostWithReleaseAgeNowEnv);

    let tmp = tempdir().expect("workspace tempdir");
    fs::write(
        tmp.path().join("pnpm-workspace.yaml"),
        "minimumReleaseAgeNow: '2026-01-01T00:00:00Z'\n",
    )
    .expect("write to pnpm-workspace.yaml");
    let config = Config::new().current::<HostWithReleaseAgeNowEnv>(tmp.path()).expect("loads");
    assert_eq!(config.minimum_release_age_now, Some(time("2026-03-01T00:00:00Z")));
}

#[test]
fn an_invalid_minimum_release_age_now_env_var_is_ignored() {
    struct HostWithBadReleaseAgeNowEnv;
    impl EnvVar for HostWithBadReleaseAgeNowEnv {
        fn var(name: &str) -> Option<String> {
            match name {
                "PNPM_CONFIG_MINIMUM_RELEASE_AGE_NOW" => Some("2026-03-01".to_owned()),
                _ => safe_host_var(name),
            }
        }
    }
    impl EnvVarOs for HostWithBadReleaseAgeNowEnv {
        fn var_os(_: &str) -> Option<OsString> {
            None
        }
    }
    impl GetHomeDir for HostWithBadReleaseAgeNowEnv {
        fn home_dir() -> Option<PathBuf> {
            None
        }
    }
    inert_link_probe!(HostWithBadReleaseAgeNowEnv);
    host_current_dir!(HostWithBadReleaseAgeNowEnv);

    let tmp = tempdir().expect("workspace tempdir");
    fs::write(
        tmp.path().join("pnpm-workspace.yaml"),
        "minimumReleaseAgeNow: '2026-01-01T00:00:00Z'\n",
    )
    .expect("write to pnpm-workspace.yaml");
    let config = Config::new().current::<HostWithBadReleaseAgeNowEnv>(tmp.path()).expect("loads");
    assert_eq!(config.minimum_release_age_now, Some(time("2026-01-01T00:00:00Z")));
}

#[test]
fn release_age_now_is_the_current_time_when_unset() {
    let before = Utc::now();
    let now = Config::default().release_age_now();
    assert!(before <= now && now <= Utc::now(), "{now} is not the current time");
}

#[test]
fn accepts_a_minimum_release_age_now_a_few_minutes_ahead_of_this_clock() {
    let ahead = (Utc::now() + chrono::Duration::minutes(5)).to_rfc3339();
    let config = config_from_workspace_yaml(&format!("minimumReleaseAgeNow: '{ahead}'\n"));
    assert!(config.minimum_release_age_now.is_some(), "clock skew of 5 minutes is allowed");
}

#[test]
fn release_age_now_applies_only_when_a_window_counts_back_from_it() {
    let now = Some(time("2026-01-01T00:00:00Z"));
    let applies = |minimum_release_age, trust_policy_ignore_after, minimum_release_age_now| {
        Config {
            minimum_release_age,
            minimum_release_age_now,
            trust_policy: TrustPolicy::NoDowngrade,
            trust_policy_ignore_after,
            ..Config::default()
        }
        .release_age_now_applies()
    };
    assert!(applies(Some(1440), None, now), "age cutoff");
    assert!(applies(Some(0), Some(60), now), "trust window");
    let trust_off = Config {
        minimum_release_age: Some(0),
        minimum_release_age_now: now,
        trust_policy_ignore_after: Some(60),
        ..Config::default()
    };
    assert!(!trust_off.release_age_now_applies(), "a trust window with trustPolicy off");
    assert!(!applies(Some(0), None, now), "nothing counts back from it");
    assert!(!applies(Some(1440), Some(60), None), "unset");
}

#[test]
fn release_age_window_from_now_adds_the_lag_of_minimum_release_age_now() {
    let config = Config {
        minimum_release_age: Some(1440),
        minimum_release_age_now: Some(Utc::now() - chrono::Duration::days(2)),
        ..Config::default()
    };
    let window = config.release_age_window_from_now().expect("age check is on");
    assert!((1440 + 2 * 1440..=1440 + 2 * 1440 + 1).contains(&window), "window {window}");
}
