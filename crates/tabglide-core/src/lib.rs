//! OS-independent decisions. Callers supply context, configuration and monotonic time.
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

#[derive(Clone, Debug)]
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
pub enum InputEvent {
    Wheel { direction: WheelDirection },
    RefocusTimerElapsed { generation: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RestoreFocus {
    pub window: WindowId,
    pub deadline: Instant,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    SwitchTab {
        target_window: WindowId,
        direction: TabDirection,
        restore_focus: Option<RestoreFocus>,
    },
    RestoreFocus {
        window: WindowId,
        generation: u64,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RefocusState {
    #[default]
    Idle,
    Pending(RestoreFocus),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AppState {
    pub refocus: RefocusState,
    generation: u64,
}

impl AppState {
    pub fn cancel_refocus(&mut self) {
        self.refocus = RefocusState::Idle;
        self.generation = self.generation.wrapping_add(1);
    }
}

pub fn process_event(
    event: InputEvent,
    context: Option<&WindowContext>,
    config: &CoreConfig,
    state: &mut AppState,
    now: Instant,
) -> Option<Command> {
    if !config.focus_return {
        state.cancel_refocus();
    }
    match event {
        InputEvent::RefocusTimerElapsed { generation } => {
            let RefocusState::Pending(pending) = state.refocus else {
                return None;
            };
            if pending.generation != generation || now < pending.deadline {
                return None;
            }
            state.refocus = RefocusState::Idle;
            Some(Command::RestoreFocus {
                window: pending.window,
                generation,
            })
        }
        InputEvent::Wheel { direction } => {
            let context = context?;
            if !config
                .allowed_applications
                .contains(&context.hovered_application)
                || !(0..=config.activation_region_height)
                    .contains(&context.pointer_y_from_monitor_top_px)
                || (!config.focus_unfocused_window
                    && context.focused_window != Some(context.hovered_window))
            {
                return None;
            }
            let original_window = match state.refocus {
                RefocusState::Pending(pending) => Some(pending.window),
                RefocusState::Idle
                    if config.focus_return
                        && context.focused_window != Some(context.hovered_window) =>
                {
                    context.focused_window
                }
                RefocusState::Idle => None,
            };
            let restore_focus = original_window.and_then(|window| {
                let deadline = now.checked_add(config.focus_return_delay)?;
                state.generation = state.generation.wrapping_add(1);
                Some(RestoreFocus {
                    window,
                    deadline,
                    generation: state.generation,
                })
            });
            state.refocus = restore_focus.map_or(RefocusState::Idle, RefocusState::Pending);
            Some(Command::SwitchTab {
                target_window: context.hovered_window,
                direction: match direction {
                    WheelDirection::Up => TabDirection::Previous,
                    WheelDirection::Down => TabDirection::Next,
                },
                restore_focus,
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
    ) -> Option<Command> {
        process_event(
            InputEvent::Wheel {
                direction: WheelDirection::Down,
            },
            Some(context),
            config,
            state,
            now,
        )
    }

    #[test]
    fn directions_and_inclusive_region() {
        let (config, context, mut state, now) = fixture();
        for (direction, expected) in [
            (WheelDirection::Up, TabDirection::Previous),
            (WheelDirection::Down, TabDirection::Next),
        ] {
            assert!(
                matches!(process_event(InputEvent::Wheel { direction }, Some(&context), &config, &mut state, now),
                Some(Command::SwitchTab { target_window: WindowId(2), direction, .. }) if direction == expected)
            );
        }
    }

    #[test]
    fn unsupported_missing_and_outside_context_do_nothing() {
        let (config, mut context, mut state, now) = fixture();
        assert_eq!(
            process_event(
                InputEvent::Wheel {
                    direction: WheelDirection::Down
                },
                None,
                &config,
                &mut state,
                now
            ),
            None
        );
        for y in [-1, 51] {
            context.pointer_y_from_monitor_top_px = y;
            assert_eq!(wheel(&config, &context, &mut state, now), None);
        }
        context.pointer_y_from_monitor_top_px = 0;
        context.hovered_application = ApplicationId::new("other.exe");
        assert_eq!(wheel(&config, &context, &mut state, now), None);
        assert_eq!(state.refocus, RefocusState::Idle);
    }

    #[test]
    fn focused_window_does_not_start_return() {
        let (config, mut context, mut state, now) = fixture();
        context.focused_window = Some(context.hovered_window);
        assert!(matches!(
            wheel(&config, &context, &mut state, now),
            Some(Command::SwitchTab {
                restore_focus: None,
                ..
            })
        ));
    }

    #[test]
    fn disabled_return_or_unknown_original_does_not_restore() {
        let (mut config, mut context, mut state, now) = fixture();
        config.focus_return = false;
        wheel(&config, &context, &mut state, now);
        assert_eq!(state.refocus, RefocusState::Idle);
        config.focus_return = true;
        context.focused_window = None;
        wheel(&config, &context, &mut state, now);
        assert_eq!(state.refocus, RefocusState::Idle);
    }

    #[test]
    fn burst_preserves_first_window_extends_deadline_and_rejects_stale_or_early_timers() {
        let (config, mut context, mut state, now) = fixture();
        wheel(&config, &context, &mut state, now);
        let RefocusState::Pending(first) = state.refocus else {
            panic!("missing return")
        };
        assert_eq!(first.window, WindowId(1));
        assert_eq!(first.deadline, now + Duration::from_millis(700));
        context.focused_window = Some(context.hovered_window);
        wheel(
            &config,
            &context,
            &mut state,
            now + Duration::from_millis(100),
        );
        let RefocusState::Pending(second) = state.refocus else {
            panic!("missing return")
        };
        assert_eq!(second.window, first.window);
        assert_eq!(second.deadline, first.deadline + Duration::from_millis(100));
        assert_ne!(first.generation, second.generation);
        for (generation, time) in [
            (first.generation, second.deadline),
            (second.generation, first.deadline),
        ] {
            assert_eq!(
                process_event(
                    InputEvent::RefocusTimerElapsed { generation },
                    None,
                    &config,
                    &mut state,
                    time
                ),
                None
            );
        }
        assert_eq!(
            process_event(
                InputEvent::RefocusTimerElapsed {
                    generation: second.generation
                },
                None,
                &config,
                &mut state,
                second.deadline
            ),
            Some(Command::RestoreFocus {
                window: WindowId(1),
                generation: second.generation
            })
        );
        assert_eq!(state.refocus, RefocusState::Idle);
    }

    #[test]
    fn ignored_scroll_does_not_extend_pending_return() {
        let (config, mut context, mut state, now) = fixture();
        wheel(&config, &context, &mut state, now);
        let before = state;
        context.pointer_y_from_monitor_top_px = 51;
        wheel(
            &config,
            &context,
            &mut state,
            now + Duration::from_millis(200),
        );
        assert_eq!(state, before);
    }

    #[test]
    fn cancellation_and_configuration_change_invalidate_return() {
        let (mut config, context, mut state, now) = fixture();
        wheel(&config, &context, &mut state, now);
        let RefocusState::Pending(first) = state.refocus else {
            panic!()
        };
        state.cancel_refocus();
        wheel(&config, &context, &mut state, now);
        assert_eq!(
            process_event(
                InputEvent::RefocusTimerElapsed {
                    generation: first.generation
                },
                None,
                &config,
                &mut state,
                first.deadline
            ),
            None
        );
        config.focus_return = false;
        wheel(&config, &context, &mut state, now);
        assert_eq!(state.refocus, RefocusState::Idle);
        config.focus_unfocused_window = false;
        assert_eq!(wheel(&config, &context, &mut state, now), None);
    }
}
