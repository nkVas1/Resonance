//! Conductor — the Resonance rules engine.
//!
//! The rule *data model* lives in `resonance_core::rules` (so config, the UI and
//! this engine all share one definition). This crate owns the *behavior*: the
//! observable [`SystemState`], trigger matching, and the decision function.
//! OS watchers that populate `SystemState` will live in submodules and are
//! wired up by the app shell (Phase 3).
//!
//! Decision model: rules are evaluated by **priority (descending), then
//! declaration order**. The first rule whose trigger matches the current
//! system state wins. A manual pin always beats rules.

pub use resonance_core::rules::{PowerSource, Restore, Rule, Trigger};

/// Snapshot of everything triggers can observe.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemState {
    /// Lower-cased image name of the foreground process, e.g. "photoshop.exe".
    pub foreground: Option<String>,
    /// Lower-cased image names of all running processes.
    pub running: Vec<String>,
    pub power: Option<PowerSource>,
    /// Profile pinned manually by the user (beats every rule).
    pub pinned_profile: Option<String>,
}

/// Does this trigger fire for the given observed state?
pub fn trigger_matches(trigger: &Trigger, state: &SystemState) -> bool {
    match trigger {
        Trigger::Foreground(image) => state
            .foreground
            .as_deref()
            .is_some_and(|fg| fg.eq_ignore_ascii_case(image)),
        Trigger::Running(image) => state.running.iter().any(|p| p.eq_ignore_ascii_case(image)),
        Trigger::Power(source) => state.power == Some(*source),
    }
}

/// The outcome of a decision pass.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Decision<'a> {
    /// User pin wins; apply this profile name.
    Pinned(&'a str),
    /// This rule matched first.
    Rule(&'a Rule),
    /// Nothing matched — the baseline (usually native) applies.
    Baseline,
}

/// Resolve which profile should be active for the given state.
///
/// Deterministic: priority desc, then declaration order. Stable across runs.
pub fn decide<'a>(rules: &'a [Rule], state: &'a SystemState) -> Decision<'a> {
    if let Some(pinned) = state.pinned_profile.as_deref() {
        return Decision::Pinned(pinned);
    }
    let mut indexed: Vec<(usize, &Rule)> = rules.iter().enumerate().collect();
    indexed.sort_by_key(|(i, r)| (-r.priority, *i));
    for (_, rule) in indexed {
        if trigger_matches(&rule.trigger, state) {
            return Decision::Rule(rule);
        }
    }
    Decision::Baseline
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(name: &str, trigger: Trigger, profile: &str, priority: i32) -> Rule {
        Rule {
            name: name.into(),
            trigger,
            profile: profile.into(),
            priority,
            restore: Restore::OnExit,
        }
    }

    fn state() -> SystemState {
        SystemState {
            foreground: Some("photoshop.exe".into()),
            running: vec!["photoshop.exe".into(), "steam.exe".into()],
            power: Some(PowerSource::Ac),
            pinned_profile: None,
        }
    }

    #[test]
    fn foreground_beats_nothing() {
        let rules = [rule(
            "ps",
            Trigger::Foreground("Photoshop.exe".into()),
            "fifth",
            0,
        )];
        assert_eq!(decide(&rules, &state()), Decision::Rule(&rules[0]));
    }

    #[test]
    fn higher_priority_wins_regardless_of_order() {
        let rules = [
            rule(
                "ps",
                Trigger::Foreground("photoshop.exe".into()),
                "fifth",
                0,
            ),
            rule(
                "battery",
                Trigger::Power(PowerSource::Ac),
                "fundamental",
                100,
            ),
        ];
        let s = state();
        let Decision::Rule(winner) = decide(&rules, &s) else {
            panic!("expected rule")
        };
        assert_eq!(winner.name, "battery");
    }

    #[test]
    fn equal_priority_falls_back_to_declaration_order() {
        let rules = [
            rule("first", Trigger::Running("steam.exe".into()), "octave", 5),
            rule(
                "second",
                Trigger::Foreground("photoshop.exe".into()),
                "fifth",
                5,
            ),
        ];
        let s = state();
        let Decision::Rule(winner) = decide(&rules, &s) else {
            panic!("expected rule")
        };
        assert_eq!(winner.name, "first");
    }

    #[test]
    fn pin_beats_everything() {
        let rules = [rule(
            "battery",
            Trigger::Power(PowerSource::Ac),
            "fundamental",
            i32::MAX,
        )];
        let mut s = state();
        s.pinned_profile = Some("octave".into());
        assert_eq!(decide(&rules, &s), Decision::Pinned("octave"));
    }

    #[test]
    fn no_match_is_baseline() {
        let rules = [rule(
            "battery",
            Trigger::Power(PowerSource::Battery),
            "fundamental",
            0,
        )];
        assert_eq!(decide(&rules, &state()), Decision::Baseline);
    }

    #[test]
    fn trigger_matching_is_case_insensitive() {
        let s = state();
        assert!(trigger_matches(&Trigger::Foreground("PHOTOSHOP.EXE".into()), &s));
        assert!(trigger_matches(&Trigger::Running("Steam.exe".into()), &s));
        assert!(!trigger_matches(&Trigger::Running("blender.exe".into()), &s));
    }
}
