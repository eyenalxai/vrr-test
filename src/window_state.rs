use crate::app::Application;
use cursor_icon::CursorIcon;
use softbuffer::Surface;
use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::info;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::keyboard::ModifiersState;
use winit::raw_window_handle::DisplayHandle;
use winit::window::{Fullscreen, Window};
pub struct WindowState {
    surface: Surface<DisplayHandle<'static>, Arc<Window>>,
    pub(crate) window: Arc<Window>,

    cursor_position: Option<PhysicalPosition<f64>>,
    pub(crate) modifiers: ModifiersState,
    occluded: bool,
}

impl WindowState {
    pub(crate) fn new(app: &Application, window: Window) -> Result<Self, Box<dyn Error>> {
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

    pub(crate) fn toggle_fullscreen(&self) {
        #[cfg(target_os = "linux")]
        let fullscreen_option = Some(Fullscreen::Borderless(None));

        #[cfg(target_os = "windows")]
        let fullscreen_option = {
            let current_monitor = self
                .window
                .current_monitor()
                .expect("Failed to get current monitor");
            let current_video_mode = current_monitor
                .video_modes()
                .max_by_key(|mode| {
                    (
                        mode.size().width,
                        mode.size().height,
                        mode.refresh_rate_millihertz(),
                    )
                })
                .expect("Failed to get max video mode");

            Some(Fullscreen::Exclusive(current_video_mode))
        };

        let fullscreen = if self.window.fullscreen().is_some() {
            info!("Exiting fullscreen");
            None
        } else {
            #[cfg(target_os = "windows")]
            if let Some(Fullscreen::Exclusive(video_mode)) = &fullscreen_option {
                let mode = video_mode.size();
                let refresh_rate = video_mode.refresh_rate_millihertz() / 1000; // Convert millihertz to hertz
                info!(
                    "Entering fullscreen: {}x{}@{}Hz",
                    mode.width, mode.height, refresh_rate
                );
            }
            #[cfg(target_os = "linux")]
            {
                info!("Entering fullscreen: Borderless");
            }
            fullscreen_option
        };

        self.window.set_fullscreen(fullscreen);
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
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

    pub(crate) fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
        if !occluded {
            self.window.request_redraw();
        }
    }

    pub(crate) fn draw(&mut self) -> Result<(), Box<dyn Error>> {
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
