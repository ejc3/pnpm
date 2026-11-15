//! The `minimumReleaseAgeNow` setting.

use crate::{Config, TrustPolicy};
use chrono::{DateTime, Utc};
use pnpm_resolving_resolver_base::lag_minutes;

impl Config {
    /// The time `minimumReleaseAge` and `trustPolicyIgnoreAfter` count back
    /// from: [`Self::minimum_release_age_now`] when it is set, the current
    /// time otherwise.
    #[must_use]
    pub fn release_age_now(&self) -> DateTime<Utc> {
        self.minimum_release_age_now.unwrap_or_else(Utc::now)
    }

    /// Whether [`Self::minimum_release_age_now`] changes anything: it is set,
    /// and a `minimumReleaseAge` cutoff or, under `trustPolicy: no-downgrade`,
    /// a `trustPolicyIgnoreAfter` window counts back from it.
    #[must_use]
    pub fn release_age_now_applies(&self) -> bool {
        let trust_window = self.trust_policy == TrustPolicy::NoDowngrade
            && self.trust_policy_ignore_after.is_some();
        self.minimum_release_age_now.is_some()
            && (self.resolved_minimum_release_age().is_some() || trust_window)
    }

    /// [`Self::resolved_minimum_release_age`] measured back from the current
    /// time: the window plus how far [`Self::minimum_release_age_now`] lags
    /// the clock. A pnpr package document dates versions only within a
    /// window measured from its last publish, so this is what it must cover.
    #[must_use]
    pub fn release_age_window_from_now(&self) -> Option<u64> {
        let lag = match self.minimum_release_age_now {
            Some(time) => lag_minutes(time, Utc::now()),
            None => 0,
        };
        self.resolved_minimum_release_age()
            .map(|minutes| minutes.saturating_add(lag))
    }
}
