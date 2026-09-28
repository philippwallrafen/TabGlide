//! A private execution seam, not a shared cross-platform backend abstraction.
use tabglide_core::{Command, TabDirection, WindowId};

pub(crate) trait WindowOperations {
    fn is_valid(&self, window: WindowId) -> bool;
    fn can_restore(&self, window: WindowId) -> bool;
    fn foreground(&self) -> Option<WindowId>;
    fn activate(&mut self, window: WindowId);
    fn send_tab(&mut self, direction: TabDirection) -> bool;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    Completed,
    InvalidWindow,
    FocusDenied,
    InputFailed,
    RestoreSuppressed,
}

pub(crate) fn execute(command: Command, windows: &mut impl WindowOperations) -> Outcome {
    let target = match command {
        Command::SwitchTab { target_window, .. } => target_window,
        Command::RestoreFocus { window, .. } => window,
    };
    if !windows.is_valid(target) {
        return Outcome::InvalidWindow;
    }
    if matches!(command, Command::RestoreFocus { .. }) && !windows.can_restore(target) {
        return Outcome::RestoreSuppressed;
    }
    if windows.foreground() != Some(target) {
        windows.activate(target);
    }
    if !windows.is_valid(target) || windows.foreground() != Some(target) {
        return Outcome::FocusDenied;
    }
    if let Command::SwitchTab { direction, .. } = command
        && !windows.send_tab(direction)
    {
        return Outcome::InputFailed;
    }
    Outcome::Completed
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeWindows {
        valid: bool,
        vanish_on_activation: bool,
        excluded_original: Option<WindowId>,
        focused: Option<WindowId>,
        allow_focus: bool,
        input_succeeds: bool,
        activations: u32,
        sent: Vec<TabDirection>,
    }
    impl Default for FakeWindows {
        fn default() -> Self {
            Self {
                valid: true,
                vanish_on_activation: false,
                excluded_original: None,
                focused: None,
                allow_focus: true,
                input_succeeds: true,
                activations: 0,
                sent: vec![],
            }
        }
    }
    impl WindowOperations for FakeWindows {
        fn is_valid(&self, _: WindowId) -> bool {
            self.valid
        }
        fn foreground(&self) -> Option<WindowId> {
            self.focused
        }
        fn can_restore(&self, window: WindowId) -> bool {
            self.excluded_original != Some(window)
        }
        fn activate(&mut self, target: WindowId) {
            self.activations += 1;
            if self.vanish_on_activation {
                self.valid = false;
            }
            if self.allow_focus {
                self.focused = Some(target);
            }
        }
        fn send_tab(&mut self, direction: TabDirection) -> bool {
            self.sent.push(direction);
            self.input_succeeds
        }
    }
    fn switch() -> Command {
        Command::SwitchTab {
            target_window: WindowId(7),
            direction: TabDirection::Next,
            restore_focus: None,
        }
    }

    #[test]
    fn successful_activation_sends_input() {
        let mut windows = FakeWindows::default();
        assert_eq!(execute(switch(), &mut windows), Outcome::Completed);
        assert_eq!(windows.activations, 1);
        assert_eq!(windows.sent, [TabDirection::Next]);
    }
    #[test]
    fn already_focused_does_not_activate() {
        let mut windows = FakeWindows {
            focused: Some(WindowId(7)),
            ..Default::default()
        };
        assert_eq!(execute(switch(), &mut windows), Outcome::Completed);
        assert_eq!(windows.activations, 0);
    }
    #[test]
    fn denied_focus_never_sends_input() {
        let mut windows = FakeWindows {
            allow_focus: false,
            ..Default::default()
        };
        assert_eq!(execute(switch(), &mut windows), Outcome::FocusDenied);
        assert!(windows.sent.is_empty());
    }
    #[test]
    fn vanished_window_is_not_restored() {
        let mut windows = FakeWindows {
            valid: false,
            ..Default::default()
        };
        assert_eq!(
            execute(
                Command::RestoreFocus {
                    window: WindowId(7),
                    generation: 1
                },
                &mut windows
            ),
            Outcome::InvalidWindow
        );
        assert_eq!(windows.activations, 0);
    }
    #[test]
    fn input_failure_is_reported() {
        let mut windows = FakeWindows {
            input_succeeds: false,
            ..Default::default()
        };
        assert_eq!(execute(switch(), &mut windows), Outcome::InputFailed);
    }

    #[test]
    fn target_disappearing_during_activation_receives_no_input() {
        let mut windows = FakeWindows {
            vanish_on_activation: true,
            ..Default::default()
        };
        assert_eq!(execute(switch(), &mut windows), Outcome::FocusDenied);
        assert!(windows.sent.is_empty());
    }

    #[test]
    fn explorer_origin_is_retained_across_multiple_targets_and_suppressed_at_return() {
        use std::time::{Duration, Instant};
        use tabglide_core::{
            AppState, ApplicationId, CoreConfig, InputEvent, RefocusState, WheelDirection,
            WindowContext, process_event,
        };
        let now = Instant::now();
        let config = CoreConfig {
            activation_region_height: 50,
            focus_unfocused_window: true,
            focus_return: true,
            focus_return_delay: Duration::from_millis(700),
            allowed_applications: vec![ApplicationId::new("browser.exe")],
        };
        let mut state = AppState::default();
        let explorer = WindowId(1);
        let mut windows = FakeWindows {
            focused: Some(explorer),
            excluded_original: Some(explorer),
            ..Default::default()
        };
        for target in [WindowId(2), WindowId(3)] {
            let context = WindowContext {
                hovered_window: target,
                focused_window: windows.focused,
                hovered_application: ApplicationId::new("browser.exe"),
                pointer_y_from_monitor_top_px: 0,
            };
            let command = process_event(
                InputEvent::Wheel {
                    direction: WheelDirection::Down,
                },
                Some(&context),
                &config,
                &mut state,
                now,
            )
            .unwrap();
            assert_eq!(execute(command, &mut windows), Outcome::Completed);
        }
        let RefocusState::Pending(pending) = state.refocus else {
            panic!("missing burst origin")
        };
        assert_eq!(pending.window, explorer);
        let command = process_event(
            InputEvent::RefocusTimerElapsed {
                generation: pending.generation,
            },
            None,
            &config,
            &mut state,
            pending.deadline,
        )
        .unwrap();
        assert_eq!(execute(command, &mut windows), Outcome::RestoreSuppressed);
        assert_eq!(windows.focused, Some(WindowId(3)));
        assert_eq!(windows.activations, 2);
        assert_eq!(state.refocus, RefocusState::Idle);
    }
}
