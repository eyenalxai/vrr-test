use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::{fmt, mem};

use ::tracing::{error, info};
use cursor_icon::CursorIcon;
use softbuffer::{Context, Surface};

use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState};
use winit::window::{Fullscreen, Icon, Window, WindowId};

#[cfg(any(x11_platform, wayland_platform))]
use winit::platform::startup_notify::{
    self, EventLoopExtStartupNotify, WindowAttributesExtStartupNotify, WindowExtStartupNotify,
};
use winit::raw_window_handle::{DisplayHandle, HasDisplayHandle};

#[path = "tracing.rs"]
mod tracing;

fn main() -> Result<(), Box<dyn Error>> {
    tracing::init();

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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum UserEvent {
    WakeUp,
}

struct Application {
    icon: Icon,
    windows: HashMap<WindowId, WindowState>,
    context: Option<Context<DisplayHandle<'static>>>,
}

impl Application {
    fn new<T>(event_loop: &EventLoop<T>) -> Self {
        // SAFETY: we drop the context right before the event loop is stopped, thus making it safe.
        let context = Some(
            Context::new(unsafe {
                mem::transmute::<DisplayHandle<'_>, DisplayHandle<'static>>(
                    event_loop.display_handle().unwrap(),
                )
            })
            .unwrap(),
        );

        // You'll have to choose an icon size at your own discretion. On X11, the desired size
        // varies by WM, and on Windows, you still have to account for screen scaling. Here
        // we use 32px, since it seems to work well enough in most cases. Be careful about
        // going too high, or you'll be bitten by the low-quality downscaling built into the
        // WM.
        let icon = load_icon(include_bytes!("data/icon.png"));

        Self {
            context,
            icon,
            windows: Default::default(),
        }
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        _tab_id: Option<String>,
    ) -> Result<WindowId, Box<dyn Error>> {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title("VRR Test")
            .with_transparent(true)
            .with_window_icon(Some(self.icon.clone()));

        let window = event_loop.create_window(window_attributes)?;

        let window_state = WindowState::new(self, window)?;
        let window_id = window_state.window.id();
        info!("Created new window with id={window_id:?}");
        self.windows.insert(window_id, window_state);
        Ok(window_id)
    }

    fn handle_action(&mut self, window_id: WindowId, action: Action) {
        let window = self.windows.get_mut(&window_id).unwrap();
        info!("Executing action: {action:?}");
        match action {
            Action::CloseWindow => {
                let _ = self.windows.remove(&window_id);
            }
            Action::ToggleFullscreen => window.toggle_fullscreen(),
        }
    }

    fn dump_monitors(&self, event_loop: &ActiveEventLoop) {
        info!("Monitors information");
        let primary_monitor = event_loop.primary_monitor();
        for monitor in event_loop.available_monitors() {
            let intro = if primary_monitor.as_ref() == Some(&monitor) {
                "Primary monitor"
            } else {
                "Monitor"
            };

            if let Some(name) = monitor.name() {
                info!("{intro}: {name}");
            } else {
                info!("{intro}: [no name]");
            }

            let PhysicalSize { width, height } = monitor.size();
            info!(
                "  Current mode: {width}x{height}{}",
                if let Some(m_hz) = monitor.refresh_rate_millihertz() {
                    format!(" @ {}.{} Hz", m_hz / 1000, m_hz % 1000)
                } else {
                    String::new()
                }
            );

            let PhysicalPosition { x, y } = monitor.position();
            info!("  Position: {x},{y}");

            info!("  Scale factor: {}", monitor.scale_factor());

            info!("  Available modes (width x height x bit-depth):");
            for mode in monitor.video_modes() {
                let PhysicalSize { width, height } = mode.size();
                let bits = mode.bit_depth();
                let m_hz = mode.refresh_rate_millihertz();
                info!(
                    "    {width}x{height}x{bits} @ {}.{} Hz",
                    m_hz / 1000,
                    m_hz % 1000
                );
            }
        }
    }

    fn process_key_binding(key: &str, mods: &ModifiersState) -> Option<Action> {
        KEY_BINDINGS.iter().find_map(|binding| {
            binding
                .is_triggered_by(&key, mods)
                .then_some(binding.action)
        })
    }

    fn print_help(&self) {
        info!("Keyboard bindings:");
        for binding in KEY_BINDINGS {
            info!(
                "{}{:<10} - {} ({})",
                modifiers_to_string(binding.mods),
                binding.trigger,
                binding.action,
                binding.action.help(),
            );
        }
    }
}

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

struct WindowState {
    surface: Surface<DisplayHandle<'static>, Arc<Window>>,
    window: Arc<Window>,

    cursor_position: Option<PhysicalPosition<f64>>,
    modifiers: ModifiersState,
    occluded: bool,
}

impl WindowState {
    fn new(app: &Application, window: Window) -> Result<Self, Box<dyn Error>> {
        let window = Arc::new(window);
        window.set_cursor_visible(false);

        let surface = Surface::new(app.context.as_ref().unwrap(), Arc::clone(&window))?;

        window.set_cursor(CursorIcon::Default);

        let size = window.inner_size();
        let mut state = Self {
            surface,
            window,
            cursor_position: Default::default(),
            modifiers: Default::default(),
            occluded: Default::default(),
        };

        state.resize(size);
        Ok(state)
    }

    pub fn cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = Some(position);
    }

    pub fn cursor_left(&mut self) {
        self.cursor_position = None;
    }

    fn toggle_fullscreen(&self) {
        let fullscreen = if self.window.fullscreen().is_some() {
            None
        } else {
            Some(Fullscreen::Borderless(None))
        };

        self.window.set_fullscreen(fullscreen);
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        {
            let (width, height) = match (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
            {
                (Some(width), Some(height)) => (width, height),
                _ => return,
            };
            self.surface
                .resize(width, height)
                .expect("failed to resize inner buffer");
        }
        self.window.request_redraw();
    }

    fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
        if !occluded {
            self.window.request_redraw();
        }
    }

    fn draw(&mut self) -> Result<(), Box<dyn Error>> {
        if self.occluded {
            info!("Skipping drawing occluded window={:?}", self.window.id());
            return Ok(());
        }

        let buffer = self.surface.buffer_mut()?;
        self.window.pre_present_notify();
        buffer.present()?;
        Ok(())
    }
}

struct Binding<T: Eq> {
    trigger: T,
    mods: ModifiersState,
    action: Action,
}

impl<T: Eq> Binding<T> {
    const fn new(trigger: T, mods: ModifiersState, action: Action) -> Self {
        Self {
            trigger,
            mods,
            action,
        }
    }

    fn is_triggered_by(&self, trigger: &T, mods: &ModifiersState) -> bool {
        &self.trigger == trigger && &self.mods == mods
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    CloseWindow,
    ToggleFullscreen,
}

impl Action {
    fn help(&self) -> &'static str {
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

fn load_icon(bytes: &[u8]) -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(bytes).unwrap().into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

fn modifiers_to_string(mods: ModifiersState) -> String {
    let mut mods_line = String::new();
    // Always add + since it's printed as a part of the bindings.
    for (modifier, desc) in [(ModifiersState::CONTROL, "Ctrl+")] {
        if !mods.contains(modifier) {
            continue;
        }

        mods_line.push_str(desc);
    }
    mods_line
}

const KEY_BINDINGS: &[Binding<&'static str>] = &[
    Binding::new("Q", ModifiersState::CONTROL, Action::CloseWindow),
    Binding::new("F", ModifiersState::CONTROL, Action::ToggleFullscreen),
];
