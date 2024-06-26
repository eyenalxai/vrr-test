use std::error::Error;
use std::fmt::Debug;

use winit::event_loop::EventLoop;

use crate::app::Application;
use crate::tracing::init_tracing;

mod actions;
mod app;
mod bindings;
mod event_handling;
mod tracing;
mod utils;
mod window_state;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum UserEvent {
    WakeUp,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let _event_loop_proxy = event_loop.create_proxy();

    // Wire the user event from another thread.
    std::thread::spawn(move || {
        // Wake up the `event_loop` once every second and dispatch a custom event
        // from a different thread.
        loop {
            let _ = _event_loop_proxy.send_event(UserEvent::WakeUp);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    let mut state = Application::new(&event_loop);

    event_loop.run_app(&mut state).map_err(Into::into)
}
