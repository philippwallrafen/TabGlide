//! OS-independent TabGlide policy and state machine.
//!
//! Platform adapters translate native input/state into `Event` values and execute returned
//! `Effect` values. The core owns sequencing: an unfocused target must be confirmed focused
//! before tab input is requested, and focus-return state is committed or rolled back here.
#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

/// An opaque identity assigned and validated by the platform adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct WindowId(pub u128);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationId(String);

impl ApplicationId {
    pub fn new(name: &str) -> Self {
        Self(name.trim().to_lowercase())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RequestId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WheelDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabDirection {
    Previous,
    Next,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowContext {
    pub hovered_window: WindowId,
    pub focused_window: Option<WindowId>,
    pub hovered_application: ApplicationId,
    pub pointer_y_from_monitor_top_px: i32,
}

#[derive(Clone, Debug)]
pub struct CoreConfig {
    pub activation_region_height: i32,
    pub focus_unfocused_window: bool,
    pub focus_return: bool,
    pub focus_return_delay: Duration,
    pub allowed_applications: Vec<ApplicationId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusOutcome {
    Confirmed,
    Rejected,
    Unconfirmed,
    InvalidWindow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabInputOutcome {
    Sent,
    Failed,
    InvalidWindow,
    FocusLost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    Wheel {
        direction: WheelDirection,
        context: WindowContext,
    },
    FocusResult {
        request_id: RequestId,
        outcome: FocusOutcome,
    },
    TabInputResult {
        request_id: RequestId,
        outcome: TabInputOutcome,
    },
    RefocusTimerElapsed {
        generation: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestoreFocus {
    pub window: WindowId,
    pub deadline: Instant,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    RequestFocus {
        request_id: RequestId,
        window: WindowId,
    },
    SendTab {
        request_id: RequestId,
        target_window: WindowId,
        direction: TabDirection,
    },
    RestoreFocus {
        window: WindowId,
        generation: u64,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum RefocusState {
    #[default]
    Idle,
    Pending(RestoreFocus),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SwitchStage {
    AwaitingFocus,
    AwaitingTabInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingSwitch {
    request_id: RequestId,
    target_window: WindowId,
    direction: TabDirection,
    previous_refocus: RefocusState,
    desired_refocus: RefocusState,
    focus_was_confirmed: bool,
    stage: SwitchStage,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AppState {
    refocus: RefocusState,
    pending_switch: Option<PendingSwitch>,
    refocus_generation: u64,
    request_generation: u64,
}

impl AppState {
    /// Cancels all pending product behavior. Platform hosts call this on disable/reload.
    pub fn cancel_refocus(&mut self) {
        self.refocus = RefocusState::Idle;
        self.pending_switch = None;
        self.refocus_generation = self.refocus_generation.wrapping_add(1);
    }

    /// The host uses this only to schedule the platform timer; policy remains in the core.
    pub fn pending_refocus(&self) -> Option<RestoreFocus> {
        match self.refocus {
            RefocusState::Idle => None,
            RefocusState::Pending(pending) => Some(pending),
        }
    }

    fn next_request_id(&mut self) -> RequestId {
        self.request_generation = self.request_generation.wrapping_add(1);
        RequestId(self.request_generation)
    }

    fn desired_refocus(
        &mut self,
        original_window: Option<WindowId>,
        config: &CoreConfig,
        now: Instant,
    ) -> RefocusState {
        if !config.focus_return {
            return RefocusState::Idle;
        }
        let Some(window) = original_window else {
            return RefocusState::Idle;
        };
        let Some(deadline) = now.checked_add(config.focus_return_delay) else {
            return RefocusState::Idle;
        };
        self.refocus_generation = self.refocus_generation.wrapping_add(1);
        RefocusState::Pending(RestoreFocus {
            window,
            deadline,
            generation: self.refocus_generation,
        })
    }
}

fn tab_direction(direction: WheelDirection) -> TabDirection {
    match direction {
        WheelDirection::Up => TabDirection::Previous,
        WheelDirection::Down => TabDirection::Next,
    }
}

/// Processes one semantic event and returns at most one semantic platform effect.
///
/// Effects that need a result are fed back as another `Event`. A platform adapter should drive
/// that short chain to completion before processing the next wheel event. This keeps sequencing
/// policy in the core while native APIs remain in the adapter.
pub fn process_event(
    event: Event,
    config: &CoreConfig,
    state: &mut AppState,
    now: Instant,
) -> Option<Effect> {
    if !config.focus_return {
        state.refocus = RefocusState::Idle;
    }

    match event {
        Event::Wheel { direction, context } => {
            if state.pending_switch.is_some()
                || !config
                    .allowed_applications
                    .contains(&context.hovered_application)
                || !(0..=config.activation_region_height)
                    .contains(&context.pointer_y_from_monitor_top_px)
                || (!config.focus_unfocused_window
                    && context.focused_window != Some(context.hovered_window))
            {
                return None;
            }

            let previous_refocus = state.refocus;
            let original_window = match previous_refocus {
                RefocusState::Pending(pending) => Some(pending.window),
                RefocusState::Idle
                    if config.focus_return
                        && context.focused_window != Some(context.hovered_window) =>
                {
                    context.focused_window
                }
                RefocusState::Idle => None,
            };
            let desired_refocus = state.desired_refocus(original_window, config, now);
            let request_id = state.next_request_id();
            let direction = tab_direction(direction);

            if context.focused_window == Some(context.hovered_window) {
                state.refocus = desired_refocus;
                state.pending_switch = Some(PendingSwitch {
                    request_id,
                    target_window: context.hovered_window,
                    direction,
                    previous_refocus,
                    desired_refocus,
                    focus_was_confirmed: false,
                    stage: SwitchStage::AwaitingTabInput,
                });
                return Some(Effect::SendTab {
                    request_id,
                    target_window: context.hovered_window,
                    direction,
                });
            }

            state.pending_switch = Some(PendingSwitch {
                request_id,
                target_window: context.hovered_window,
                direction,
                previous_refocus,
                desired_refocus,
                focus_was_confirmed: false,
                stage: SwitchStage::AwaitingFocus,
            });
            Some(Effect::RequestFocus {
                request_id,
                window: context.hovered_window,
            })
        }
        Event::FocusResult {
            request_id,
            outcome,
        } => {
            let Some(mut pending) = state.pending_switch else {
                return None;
            };
            if pending.request_id != request_id || pending.stage != SwitchStage::AwaitingFocus {
                return None;
            }

            if outcome != FocusOutcome::Confirmed {
                state.refocus = if config.focus_return {
                    pending.previous_refocus
                } else {
                    RefocusState::Idle
                };
                state.pending_switch = None;
                return None;
            }

            pending.focus_was_confirmed = true;
            pending.stage = SwitchStage::AwaitingTabInput;
            state.refocus = if config.focus_return {
                pending.desired_refocus
            } else {
                RefocusState::Idle
            };
            state.pending_switch = Some(pending);
            Some(Effect::SendTab {
                request_id,
                target_window: pending.target_window,
                direction: pending.direction,
            })
        }
        Event::TabInputResult {
            request_id,
            outcome,
        } => {
            let Some(pending) = state.pending_switch else {
                return None;
            };
            if pending.request_id != request_id || pending.stage != SwitchStage::AwaitingTabInput {
                return None;
            }

            if matches!(
                outcome,
                TabInputOutcome::InvalidWindow | TabInputOutcome::FocusLost
            ) && !pending.focus_was_confirmed
            {
                state.refocus = if config.focus_return {
                    pending.previous_refocus
                } else {
                    RefocusState::Idle
                };
            }
            state.pending_switch = None;
            None
        }
        Event::RefocusTimerElapsed { generation } => {
            if state.pending_switch.is_some() {
                return None;
            }
            let RefocusState::Pending(pending) = state.refocus else {
                return None;
            };
            if pending.generation != generation || now < pending.deadline {
                return None;
            }
            state.refocus = RefocusState::Idle;
            Some(Effect::RestoreFocus {
                window: pending.window,
                generation,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (CoreConfig, WindowContext, AppState, Instant) {
        (
            CoreConfig {
                activation_region_height: 50,
                focus_unfocused_window: true,
                focus_return: true,
                focus_return_delay: Duration::from_millis(700),
                allowed_applications: vec![ApplicationId::new("chrome.exe")],
            },
            WindowContext {
                hovered_window: WindowId(2),
                focused_window: Some(WindowId(1)),
                hovered_application: ApplicationId::new("CHROME.EXE"),
                pointer_y_from_monitor_top_px: 50,
            },
            AppState::default(),
            Instant::now(),
        )
    }

    fn wheel(
        config: &CoreConfig,
        context: &WindowContext,
        state: &mut AppState,
        now: Instant,
    ) -> Option<Effect> {
        process_event(
            Event::Wheel {
                direction: WheelDirection::Down,
                context: context.clone(),
            },
            config,
            state,
            now,
        )
    }

    fn confirm_focus_and_tab(
        config: &CoreConfig,
        state: &mut AppState,
        request_id: RequestId,
        now: Instant,
    ) {
        assert!(matches!(
            process_event(
                Event::FocusResult {
                    request_id,
                    outcome: FocusOutcome::Confirmed,
                },
                config,
                state,
                now,
            ),
            Some(Effect::SendTab {
                request_id: id,
                ..
            }) if id == request_id
        ));
        assert_eq!(
            process_event(
                Event::TabInputResult {
                    request_id,
                    outcome: TabInputOutcome::Sent,
                },
                config,
                state,
                now,
            ),
            None
        );
    }

    #[test]
    fn directions_and_inclusive_region() {
        let (config, mut context, mut state, now) = fixture();
        context.focused_window = Some(context.hovered_window);
        for (direction, expected) in [
            (WheelDirection::Up, TabDirection::Previous),
            (WheelDirection::Down, TabDirection::Next),
        ] {
            let effect = process_event(
                Event::Wheel {
                    direction,
                    context: context.clone(),
                },
                &config,
                &mut state,
                now,
            );
            let Some(Effect::SendTab {
                request_id,
                direction,
                ..
            }) = effect
            else {
                panic!("expected tab effect")
            };
            assert_eq!(direction, expected);
            let _ = process_event(
                Event::TabInputResult {
                    request_id,
                    outcome: TabInputOutcome::Sent,
                },
                &config,
                &mut state,
                now,
            );
        }
    }

    #[test]
    fn unsupported_and_outside_context_do_nothing() {
        let (mut config, mut context, mut state, now) = fixture();
        for y in [-1, 51] {
            context.pointer_y_from_monitor_top_px = y;
            assert_eq!(wheel(&config, &context, &mut state, now), None);
        }
        context.pointer_y_from_monitor_top_px = 0;
        context.hovered_application = ApplicationId::new("other.exe");
        assert_eq!(wheel(&config, &context, &mut state, now), None);
        context.hovered_application = ApplicationId::new("chrome.exe");
        config.focus_unfocused_window = false;
        assert_eq!(wheel(&config, &context, &mut state, now), None);
        assert_eq!(state.pending_refocus(), None);
    }

    #[test]
    fn unfocused_target_requires_focus_confirmation_before_tab_input() {
        let (config, context, mut state, now) = fixture();
        let Some(Effect::RequestFocus { request_id, window }) =
            wheel(&config, &context, &mut state, now)
        else {
            panic!("expected focus request")
        };
        assert_eq!(window, context.hovered_window);
        assert_eq!(state.pending_refocus(), None);

        let effect = process_event(
            Event::FocusResult {
                request_id,
                outcome: FocusOutcome::Confirmed,
            },
            &config,
            &mut state,
            now + Duration::from_millis(5),
        );
        assert_eq!(
            effect,
            Some(Effect::SendTab {
                request_id,
                target_window: context.hovered_window,
                direction: TabDirection::Next,
            })
        );
        assert_eq!(state.pending_refocus().unwrap().window, WindowId(1));

        assert_eq!(
            process_event(
                Event::TabInputResult {
                    request_id,
                    outcome: TabInputOutcome::Sent,
                },
                &config,
                &mut state,
                now,
            ),
            None
        );
        assert!(state.pending_switch.is_none());
    }

    #[test]
    fn rejected_focus_restores_previous_refocus_state() {
        let (config, mut context, mut state, now) = fixture();
        let Some(Effect::RequestFocus {
            request_id: first_request,
            ..
        }) = wheel(&config, &context, &mut state, now)
        else {
            panic!()
        };
        confirm_focus_and_tab(&config, &mut state, first_request, now);
        let first = state.pending_refocus().unwrap();

        context.hovered_window = WindowId(3);
        context.focused_window = Some(WindowId(2));
        let Some(Effect::RequestFocus {
            request_id: second_request,
            ..
        }) = wheel(
            &config,
            &context,
            &mut state,
            now + Duration::from_millis(100),
        )
        else {
            panic!()
        };
        assert_eq!(
            process_event(
                Event::FocusResult {
                    request_id: second_request,
                    outcome: FocusOutcome::Rejected,
                },
                &config,
                &mut state,
                now,
            ),
            None
        );
        assert_eq!(state.pending_refocus(), Some(first));
    }

    #[test]
    fn burst_preserves_first_window_extends_deadline_and_rejects_stale_timers() {
        let (config, mut context, mut state, now) = fixture();
        let Some(Effect::RequestFocus { request_id, .. }) =
            wheel(&config, &context, &mut state, now)
        else {
            panic!()
        };
        confirm_focus_and_tab(&config, &mut state, request_id, now);
        let first = state.pending_refocus().unwrap();

        context.focused_window = Some(context.hovered_window);
        let later = now + Duration::from_millis(100);
        let Some(Effect::SendTab { request_id, .. }) = wheel(&config, &context, &mut state, later)
        else {
            panic!()
        };
        let _ = process_event(
            Event::TabInputResult {
                request_id,
                outcome: TabInputOutcome::Sent,
            },
            &config,
            &mut state,
            later,
        );
        let second = state.pending_refocus().unwrap();
        assert_eq!(second.window, first.window);
        assert_eq!(second.deadline, first.deadline + Duration::from_millis(100));
        assert_ne!(first.generation, second.generation);

        assert_eq!(
            process_event(
                Event::RefocusTimerElapsed {
                    generation: first.generation,
                },
                &config,
                &mut state,
                second.deadline,
            ),
            None
        );
        assert_eq!(
            process_event(
                Event::RefocusTimerElapsed {
                    generation: second.generation,
                },
                &config,
                &mut state,
                first.deadline,
            ),
            None
        );
        assert_eq!(
            process_event(
                Event::RefocusTimerElapsed {
                    generation: second.generation,
                },
                &config,
                &mut state,
                second.deadline,
            ),
            Some(Effect::RestoreFocus {
                window: WindowId(1),
                generation: second.generation,
            })
        );
        assert_eq!(state.pending_refocus(), None);
    }

    #[test]
    fn focus_return_disabled_still_switches_without_scheduling_restore() {
        let (mut config, context, mut state, now) = fixture();
        config.focus_return = false;
        let Some(Effect::RequestFocus { request_id, .. }) =
            wheel(&config, &context, &mut state, now)
        else {
            panic!()
        };
        assert!(matches!(
            process_event(
                Event::FocusResult {
                    request_id,
                    outcome: FocusOutcome::Confirmed,
                },
                &config,
                &mut state,
                now,
            ),
            Some(Effect::SendTab { .. })
        ));
        assert_eq!(state.pending_refocus(), None);
    }

    #[test]
    fn input_failure_after_confirmed_focus_keeps_return() {
        let (config, context, mut state, now) = fixture();
        let Some(Effect::RequestFocus { request_id, .. }) =
            wheel(&config, &context, &mut state, now)
        else {
            panic!()
        };
        assert!(matches!(
            process_event(
                Event::FocusResult {
                    request_id,
                    outcome: FocusOutcome::Confirmed,
                },
                &config,
                &mut state,
                now,
            ),
            Some(Effect::SendTab { .. })
        ));
        let pending = state.pending_refocus();
        assert_eq!(
            process_event(
                Event::TabInputResult {
                    request_id,
                    outcome: TabInputOutcome::Failed,
                },
                &config,
                &mut state,
                now,
            ),
            None
        );
        assert_eq!(state.pending_refocus(), pending);
    }

    #[test]
    fn cancel_invalidates_late_platform_feedback() {
        let (config, context, mut state, now) = fixture();
        let Some(Effect::RequestFocus { request_id, .. }) =
            wheel(&config, &context, &mut state, now)
        else {
            panic!()
        };
        state.cancel_refocus();
        assert_eq!(
            process_event(
                Event::FocusResult {
                    request_id,
                    outcome: FocusOutcome::Confirmed,
                },
                &config,
                &mut state,
                now,
            ),
            None
        );
        assert_eq!(state.pending_refocus(), None);
    }
}
