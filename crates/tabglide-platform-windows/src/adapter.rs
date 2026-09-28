//! Windows-only adapter for executing semantic core effects.
//!
//! This module does not decide product sequencing. It translates one `Effect` into Win32 work
//! and, when the core needs a result, returns the corresponding semantic `Event`.
use std::time::Duration;
use tabglide_core::{
    Effect, Event, FocusOutcome, RequestId, TabDirection, TabInputOutcome, WindowId,
};

const FOREGROUND_CONFIRMATION_TIMEOUT: Duration = Duration::from_millis(100);

pub(crate) trait WindowOperations {
    fn is_valid(&self, window: WindowId) -> bool;
    fn foreground(&self) -> Option<WindowId>;
    fn activate(&mut self, window: WindowId) -> bool;
    fn wait_for_foreground(&self, window: WindowId, timeout: Duration) -> bool;
    fn send_tab(&mut self, direction: TabDirection) -> bool;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    Completed,
    InvalidWindow,
    FocusDenied,
    FocusUnconfirmed,
    FocusLost,
    InputFailed,
}

pub(crate) struct Execution {
    pub outcome: Outcome,
    pub feedback: Option<Event>,
}

fn focus_feedback(request_id: RequestId, outcome: FocusOutcome, result: Outcome) -> Execution {
    Execution {
        outcome: result,
        feedback: Some(Event::FocusResult {
            request_id,
            outcome,
        }),
    }
}

fn tab_feedback(request_id: RequestId, outcome: TabInputOutcome, result: Outcome) -> Execution {
    Execution {
        outcome: result,
        feedback: Some(Event::TabInputResult {
            request_id,
            outcome,
        }),
    }
}

pub(crate) fn execute(effect: Effect, windows: &mut impl WindowOperations) -> Execution {
    match effect {
        Effect::RequestFocus { request_id, window } => {
            if !windows.is_valid(window) {
                return focus_feedback(
                    request_id,
                    FocusOutcome::InvalidWindow,
                    Outcome::InvalidWindow,
                );
            }
            if windows.foreground() == Some(window) {
                return focus_feedback(request_id, FocusOutcome::Confirmed, Outcome::Completed);
            }
            if !windows.activate(window) {
                return focus_feedback(request_id, FocusOutcome::Rejected, Outcome::FocusDenied);
            }
            if windows.wait_for_foreground(window, FOREGROUND_CONFIRMATION_TIMEOUT)
                && windows.is_valid(window)
                && windows.foreground() == Some(window)
            {
                return focus_feedback(request_id, FocusOutcome::Confirmed, Outcome::Completed);
            }
            if !windows.is_valid(window) {
                focus_feedback(
                    request_id,
                    FocusOutcome::InvalidWindow,
                    Outcome::InvalidWindow,
                )
            } else {
                focus_feedback(
                    request_id,
                    FocusOutcome::Unconfirmed,
                    Outcome::FocusUnconfirmed,
                )
            }
        }
        Effect::SendTab {
            request_id,
            target_window,
            direction,
        } => {
            if !windows.is_valid(target_window) {
                return tab_feedback(
                    request_id,
                    TabInputOutcome::InvalidWindow,
                    Outcome::InvalidWindow,
                );
            }
            if windows.foreground() != Some(target_window) {
                return tab_feedback(
                    request_id,
                    TabInputOutcome::FocusLost,
                    Outcome::FocusLost,
                );
            }
            if !windows.send_tab(direction) {
                return tab_feedback(
                    request_id,
                    TabInputOutcome::Failed,
                    Outcome::InputFailed,
                );
            }
            tab_feedback(request_id, TabInputOutcome::Sent, Outcome::Completed)
        }
        Effect::RestoreFocus { window, .. } => {
            if !windows.is_valid(window) {
                return Execution {
                    outcome: Outcome::InvalidWindow,
                    feedback: None,
                };
            }
            if windows.foreground() != Some(window) {
                if !windows.activate(window) {
                    return Execution {
                        outcome: Outcome::FocusDenied,
                        feedback: None,
                    };
                }
                if !windows.wait_for_foreground(window, FOREGROUND_CONFIRMATION_TIMEOUT) {
                    return Execution {
                        outcome: if windows.is_valid(window) {
                            Outcome::FocusUnconfirmed
                        } else {
                            Outcome::InvalidWindow
                        },
                        feedback: None,
                    };
                }
            }
            Execution {
                outcome: Outcome::Completed,
                feedback: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeWindows {
        valid: bool,
        vanish_on_activation: bool,
        focused: Option<WindowId>,
        activation_accepted: bool,
        confirm_foreground: bool,
        input_succeeds: bool,
        activations: u32,
        sent: Vec<TabDirection>,
    }

    impl Default for FakeWindows {
        fn default() -> Self {
            Self {
                valid: true,
                vanish_on_activation: false,
                focused: None,
                activation_accepted: true,
                confirm_foreground: true,
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

        fn activate(&mut self, target: WindowId) -> bool {
            self.activations += 1;
            if self.vanish_on_activation {
                self.valid = false;
            }
            if self.activation_accepted && self.confirm_foreground {
                self.focused = Some(target);
            }
            self.activation_accepted
        }

        fn wait_for_foreground(&self, window: WindowId, _: Duration) -> bool {
            self.focused == Some(window)
        }

        fn send_tab(&mut self, direction: TabDirection) -> bool {
            self.sent.push(direction);
            self.input_succeeds
        }
    }

    #[test]
    fn focus_request_reports_confirmation() {
        let mut windows = FakeWindows::default();
        let execution = execute(
            Effect::RequestFocus {
                request_id: RequestId(1),
                window: WindowId(7),
            },
            &mut windows,
        );
        assert_eq!(execution.outcome, Outcome::Completed);
        assert!(matches!(
            execution.feedback,
            Some(Event::FocusResult {
                request_id: RequestId(1),
                outcome: FocusOutcome::Confirmed,
            })
        ));
        assert_eq!(windows.activations, 1);
    }

    #[test]
    fn rejected_or_unconfirmed_focus_is_reported_without_tab_input() {
        let mut denied = FakeWindows {
            activation_accepted: false,
            ..Default::default()
        };
        let execution = execute(
            Effect::RequestFocus {
                request_id: RequestId(1),
                window: WindowId(7),
            },
            &mut denied,
        );
        assert_eq!(execution.outcome, Outcome::FocusDenied);
        assert!(denied.sent.is_empty());

        let mut unconfirmed = FakeWindows {
            confirm_foreground: false,
            ..Default::default()
        };
        let execution = execute(
            Effect::RequestFocus {
                request_id: RequestId(2),
                window: WindowId(7),
            },
            &mut unconfirmed,
        );
        assert_eq!(execution.outcome, Outcome::FocusUnconfirmed);
        assert!(unconfirmed.sent.is_empty());
    }

    #[test]
    fn send_tab_requires_target_to_still_be_foreground() {
        let mut windows = FakeWindows {
            focused: Some(WindowId(99)),
            ..Default::default()
        };
        let execution = execute(
            Effect::SendTab {
                request_id: RequestId(1),
                target_window: WindowId(7),
                direction: TabDirection::Next,
            },
            &mut windows,
        );
        assert_eq!(execution.outcome, Outcome::FocusLost);
        assert!(windows.sent.is_empty());
        assert!(matches!(
            execution.feedback,
            Some(Event::TabInputResult {
                outcome: TabInputOutcome::FocusLost,
                ..
            })
        ));
    }

    #[test]
    fn successful_tab_input_is_reported() {
        let mut windows = FakeWindows {
            focused: Some(WindowId(7)),
            ..Default::default()
        };
        let execution = execute(
            Effect::SendTab {
                request_id: RequestId(1),
                target_window: WindowId(7),
                direction: TabDirection::Previous,
            },
            &mut windows,
        );
        assert_eq!(execution.outcome, Outcome::Completed);
        assert_eq!(windows.sent, [TabDirection::Previous]);
        assert!(matches!(
            execution.feedback,
            Some(Event::TabInputResult {
                outcome: TabInputOutcome::Sent,
                ..
            })
        ));
    }

    #[test]
    fn vanished_target_is_reported_invalid() {
        let mut windows = FakeWindows {
            vanish_on_activation: true,
            ..Default::default()
        };
        let execution = execute(
            Effect::RequestFocus {
                request_id: RequestId(1),
                window: WindowId(7),
            },
            &mut windows,
        );
        assert_eq!(execution.outcome, Outcome::InvalidWindow);
        assert!(matches!(
            execution.feedback,
            Some(Event::FocusResult {
                outcome: FocusOutcome::InvalidWindow,
                ..
            })
        ));
    }

    #[test]
    fn vanished_restore_target_is_ignored_safely() {
        let mut windows = FakeWindows {
            valid: false,
            ..Default::default()
        };
        let execution = execute(
            Effect::RestoreFocus {
                window: WindowId(7),
                generation: 1,
            },
            &mut windows,
        );
        assert_eq!(execution.outcome, Outcome::InvalidWindow);
        assert_eq!(windows.activations, 0);
        assert!(execution.feedback.is_none());
    }
}
