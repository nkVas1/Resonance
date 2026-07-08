//! Conductor — the Resonance rules engine.
//!
//! The rule *data model* lives in `resonance_core::rules` (so config, the UI and
//! this engine all share one definition). This crate owns the *behavior*: the
//! observable [`SystemState`], trigger matching, and the decision function.
//! OS watchers that populate `SystemState` live in [`watch`]; the app shell
//! polls them, feeds [`Engine::evaluate`], and executes the returned [`Action`].
//!
//! Decision model: rules are evaluated by **priority (descending), then
//! declaration order**. The first rule whose trigger matches the current
//! system state wins. A manual pin always beats rules.

pub mod watch;

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

/// True if any rule needs the (heavier) running-process list.
pub fn needs_running(rules: &[Rule]) -> bool {
    rules
        .iter()
        .any(|r| matches!(r.trigger, Trigger::Running(_)))
}

/// What the app shell should do this tick. The engine only emits an action
/// when the desired target actually changes, so applying it is idempotent.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Action {
    /// Apply this profile (a rule matched, or the user pinned it).
    Apply { profile: String, cause: Cause },
    /// No rule matches anymore — return to the state saved before automation.
    RestoreBaseline,
    /// Nothing to do.
    Idle,
}

/// Why a profile became active — surfaced in the UI ("why is my resolution X?").
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Cause {
    Rule(String),
    Pin,
}

/// Stateful automation driver. Holds only the currently-applied target so it
/// can detect transitions; the app shell owns the baseline it restores to.
#[derive(Default)]
pub struct Engine {
    active: Option<String>,
    enabled: bool,
}

impl Engine {
    pub fn new(enabled: bool) -> Self {
        Engine {
            active: None,
            enabled,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The profile name automation currently holds active, if any.
    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// Force the engine's notion of the active profile (used when the user makes
    /// a manual switch, so the poll loop treats it as already-applied and won't
    /// re-drive or immediately revert it).
    pub fn force_active(&mut self, profile: Option<String>) {
        self.active = profile;
    }

    /// Feed the latest rules + observed state; get the transition to perform.
    ///
    /// A manual pin is honored even when automation is disabled (it is a direct
    /// user command). Rule-based switching only happens while enabled.
    pub fn evaluate(&mut self, rules: &[Rule], state: &SystemState) -> Action {
        let (target, cause): (Option<&str>, Cause) = match decide(rules, state) {
            Decision::Pinned(name) => (Some(name), Cause::Pin),
            Decision::Rule(rule) if self.enabled => {
                (Some(&rule.profile), Cause::Rule(rule.name.clone()))
            }
            Decision::Rule(_) | Decision::Baseline => (None, Cause::Pin),
        };

        match (target, self.active.as_deref()) {
            (Some(t), current) if current != Some(t) => {
                self.active = Some(t.to_string());
                Action::Apply {
                    profile: t.to_string(),
                    cause,
                }
            }
            (None, Some(_)) => {
                self.active = None;
                Action::RestoreBaseline
            }
            _ => Action::Idle,
        }
    }
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
        assert!(trigger_matches(
            &Trigger::Foreground("PHOTOSHOP.EXE".into()),
            &s
        ));
        assert!(trigger_matches(&Trigger::Running("Steam.exe".into()), &s));
        assert!(!trigger_matches(
            &Trigger::Running("blender.exe".into()),
            &s
        ));
    }

    // ---- engine transitions ----

    fn ps_rule() -> Vec<Rule> {
        vec![rule(
            "ps",
            Trigger::Foreground("photoshop.exe".into()),
            "fifth",
            0,
        )]
    }

    #[test]
    fn engine_applies_once_then_idles() {
        let mut e = Engine::new(true);
        let rules = ps_rule();
        // First match → apply.
        assert_eq!(
            e.evaluate(&rules, &state()),
            Action::Apply {
                profile: "fifth".into(),
                cause: Cause::Rule("ps".into())
            }
        );
        // Same state again → no repeat action.
        assert_eq!(e.evaluate(&rules, &state()), Action::Idle);
        assert_eq!(e.active(), Some("fifth"));
    }

    #[test]
    fn engine_restores_when_trigger_clears() {
        let mut e = Engine::new(true);
        let rules = ps_rule();
        e.evaluate(&rules, &state());
        let mut cleared = state();
        cleared.foreground = Some("explorer.exe".into());
        assert_eq!(e.evaluate(&rules, &cleared), Action::RestoreBaseline);
        assert_eq!(e.active(), None);
        // Baseline holds — no repeated restores.
        assert_eq!(e.evaluate(&rules, &cleared), Action::Idle);
    }

    #[test]
    fn disabled_engine_ignores_rules_but_honors_pin() {
        let mut e = Engine::new(false);
        let rules = ps_rule();
        assert_eq!(e.evaluate(&rules, &state()), Action::Idle);
        let mut pinned = state();
        pinned.pinned_profile = Some("octave".into());
        assert_eq!(
            e.evaluate(&rules, &pinned),
            Action::Apply {
                profile: "octave".into(),
                cause: Cause::Pin
            }
        );
    }

    #[test]
    fn engine_switches_directly_between_rule_targets() {
        let mut e = Engine::new(true);
        let rules = vec![
            rule(
                "ps",
                Trigger::Foreground("photoshop.exe".into()),
                "fifth",
                0,
            ),
            rule("game", Trigger::Foreground("game.exe".into()), "octave", 0),
        ];
        e.evaluate(&rules, &state());
        let mut gaming = state();
        gaming.foreground = Some("game.exe".into());
        assert_eq!(
            e.evaluate(&rules, &gaming),
            Action::Apply {
                profile: "octave".into(),
                cause: Cause::Rule("game".into())
            }
        );
    }
}
