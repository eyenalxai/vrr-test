use crate::actions::Action;
use winit::keyboard::ModifiersState;

pub struct Binding<T: Eq> {
    pub(crate) trigger: T,
    pub(crate) mods: ModifiersState,
    pub(crate) action: Action,
}

impl<T: Eq> Binding<T> {
    pub(crate) const fn new(trigger: T, mods: ModifiersState, action: Action) -> Self {
        Self {
            trigger,
            mods,
            action,
        }
    }

    pub(crate) fn is_triggered_by(&self, trigger: &T, mods: &ModifiersState) -> bool {
        &self.trigger == trigger && &self.mods == mods
    }
}
