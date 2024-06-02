use crate::app::Application;
use crate::UserEvent;
use tracing::error;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::WindowId;

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.dump_monitors(event_loop);

        self.create_window(event_loop, None)
            .expect("failed to create initial window");

        self.print_help();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match self.windows.get_mut(&window_id) {
            Some(window) => window,
            None => return,
        };

        match event {
            WindowEvent::Resized(size) => {
                window.resize(size);
            }

            WindowEvent::RedrawRequested => {
                if let Err(err) = window.draw() {
                    error!("Error drawing window: {err}");
                }
            }
            WindowEvent::Occluded(occluded) => {
                window.set_occluded(occluded);
            }
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                window.modifiers = modifiers.state();
            }

            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let mods = window.modifiers;

                if event.state.is_pressed() {
                    let action = if let Key::Character(ch) = event.logical_key.as_ref() {
                        Self::process_key_binding(&ch.to_uppercase(), &mods)
                    } else {
                        None
                    };

                    if let Some(action) = action {
                        self.handle_action(window_id, action);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                window.cursor_left();
            }
            WindowEvent::CursorMoved { position, .. } => {
                window.cursor_moved(position);
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.context = None;
    }
}
