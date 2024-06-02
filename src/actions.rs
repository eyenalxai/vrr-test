use crate::bindings::Binding;
use std::fmt;
use std::fmt::Debug;
use winit::keyboard::ModifiersState;

pub const KEY_BINDINGS: &[Binding<&'static str>] = &[
    Binding::new("Q", ModifiersState::CONTROL, Action::CloseWindow),
    Binding::new("F", ModifiersState::CONTROL, Action::ToggleFullscreen),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    CloseWindow,
    ToggleFullscreen,
}

impl Action {
    pub(crate) fn help(&self) -> &'static str {
        match self {
            Action::CloseWindow => "Close window",
            Action::ToggleFullscreen => "Toggle fullscreen",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self, f)
    }
}
