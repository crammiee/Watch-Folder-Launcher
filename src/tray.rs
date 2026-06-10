use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use crate::Config;

pub fn run(config: Arc<Config>, active: Arc<AtomicBool>) {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let quit_i = MenuItem::new("Quit", true, None);
    let toggle_i = MenuItem::new("Pause Watching", true, None);
    let label_i = MenuItem::new(
        &format!("Watching: {}", clip(&config.watch_folder, 38)),
        false,
        None,
    );

    let menu = Menu::new();
    menu.append_items(&[
        &label_i,
        &PredefinedMenuItem::separator(),
        &toggle_i,
        &PredefinedMenuItem::separator(),
        &quit_i,
    ])
    .expect("Failed to build tray menu");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(&format!("{} — Watching", config.app_display_name))
        .with_icon(make_icon(true))
        .build()
        .expect("Failed to create tray icon");

    let mut app = App {
        quit_id: quit_i.id().clone(),
        toggle_id: toggle_i.id().clone(),
        toggle_i,
        tray,
        active,
        config,
    };

    event_loop.run_app(&mut app).expect("Event loop error");
}

struct App {
    quit_id: tray_icon::menu::MenuId,
    toggle_id: tray_icon::menu::MenuId,
    toggle_i: MenuItem,
    tray: TrayIcon,
    active: Arc<AtomicBool>,
    config: Arc<Config>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Poll at 10 Hz — low enough to be invisible, fast enough for responsive menu.
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(100),
        ));

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.quit_id {
                self.active.store(false, Ordering::Relaxed);
                event_loop.exit();
            } else if ev.id == self.toggle_id {
                let was_watching = self.active.load(Ordering::Relaxed);
                self.active.store(!was_watching, Ordering::Relaxed);

                if was_watching {
                    self.toggle_i.set_text("Resume Watching");
                    let _ = self.tray.set_icon(Some(make_icon(false)));
                    let _ = self.tray.set_tooltip(Some(
                        &format!("{} — Paused", self.config.app_display_name),
                    ));
                } else {
                    self.toggle_i.set_text("Pause Watching");
                    let _ = self.tray.set_icon(Some(make_icon(true)));
                    let _ = self.tray.set_tooltip(Some(
                        &format!("{} — Watching", self.config.app_display_name),
                    ));
                }
            }
        }
    }
}

/// Draws a filled circle: green when watching, grey when paused.
fn make_icon(watching: bool) -> Icon {
    const SIZE: u32 = 32;
    let cx = SIZE as f32 / 2.0 - 0.5;
    let cy = SIZE as f32 / 2.0 - 0.5;
    let r = SIZE as f32 / 2.0 - 2.0;

    let fill: [u8; 4] = if watching {
        [30, 185, 90, 255]
    } else {
        [105, 105, 105, 255]
    };

    let rgba: Vec<u8> = (0..SIZE * SIZE)
        .flat_map(|i| {
            let x = (i % SIZE) as f32;
            let y = (i / SIZE) as f32;
            if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() <= r {
                fill
            } else {
                [0u8, 0, 0, 0]
            }
        })
        .collect();

    Icon::from_rgba(rgba, SIZE, SIZE).expect("Failed to create icon")
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("...{}", &s[s.len().saturating_sub(max - 3)..])
    }
}
