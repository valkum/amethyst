use crate::{config::DisplayConfig, resources::ScreenDimensions};
use amethyst_config::Config;
use amethyst_core::{
    ecs::{Resources, RunNow, System, SystemData, Write, WriteExpect},
    shrev::EventChannel,
};
use std::{path::Path, sync::Arc, mem};
use winit::{Event, EventsLoop, Window, WindowEvent};


/// System for opening and managing the window.
pub struct WindowSystem {
    window: Arc<Window>,
}

impl WindowSystem {
    pub fn from_config_path(events_loop: &EventsLoop, path: impl AsRef<Path>) -> Self {
        Self::from_config(events_loop, DisplayConfig::load(path.as_ref()))
    }

    pub fn from_config(events_loop: &EventsLoop, config: DisplayConfig) -> Self {
        let window = config
            .to_windowbuilder(events_loop)
            .build(events_loop)
            .unwrap();
        Self::new(window)
    }

    pub fn new(window: Window) -> Self {
        Self {
            window: Arc::new(window),
        }
    }

    fn manage_dimensions(&mut self, mut screen_dimensions: &mut ScreenDimensions) {
        let width = screen_dimensions.w;
        let height = screen_dimensions.h;

        // Send resource size changes to the window
        if screen_dimensions.dirty {
            self.window.set_inner_size((width, height).into());
            if screen_dimensions.fullscreen {
                self.window.set_fullscreen(Some(self.window.get_primary_monitor()));
            } else {
                self.window.set_fullscreen(None);
                if screen_dimensions.maximized {
                    self.window.set_maximized(true);
                } else {
                    self.window.set_maximized(false);
                }
            }
            screen_dimensions.dirty = false;
        }

        let hidpi = self.window.get_hidpi_factor();

        if let Some(size) = self.window.get_inner_size() {
            let (window_width, window_height): (f64, f64) = size.to_physical(hidpi).into();

            // Send window size changes to the resource
            if (window_width, window_height) != (width, height) {
                screen_dimensions.update(window_width, window_height);

                // We don't need to send the updated size of the window back to the window itself,
                // so set dirty to false.
                screen_dimensions.dirty = false;
            }
        }
        screen_dimensions.update_hidpi_factor(hidpi);
    }
}

impl<'a> System<'a> for WindowSystem {
    type SystemData = WriteExpect<'a, ScreenDimensions>;

    fn run(&mut self, mut screen_dimesnions: Self::SystemData) {
        self.manage_dimensions(&mut screen_dimesnions);
    }
    fn setup(&mut self, res: &mut Resources) {
        let (width, height) = self
            .window
            .get_inner_size()
            .expect("Window closed during initialization!")
            .into();
        let hidpi = self.window.get_hidpi_factor();
        res.insert(ScreenDimensions::new(width, height, hidpi));
        res.insert(self.window.clone());
    }
}

/// System that polls the window events and pushes them to appropriate event channels.
///
/// This system must be active for any `GameState` to receive
/// any `StateEvent::Window` event into it's `handle_event` method.
pub struct EventsLoopSystem {
    events_loop: EventsLoop,
    events: Vec<Event>,
}

impl EventsLoopSystem {
    pub fn new(events_loop: EventsLoop) -> Self {
        Self {
            events_loop,
            events: Vec::with_capacity(128),
        }
    }
}

impl<'a> RunNow<'a> for EventsLoopSystem {
    fn run_now(&mut self, res: &'a Resources) {
        let mut event_handler = <Write<'a, EventChannel<Event>>>::fetch(res);

        let events = &mut self.events;
        
        self.events_loop.poll_events(|event| {
            compress_events(events, event);
        });
        event_handler.drain_vec_write(events);
    }

    fn setup(&mut self, res: &mut Resources) {
        <Write<'a, EventChannel<Event>>>::setup(res);
    }
}


/// Input devices can sometimes generate a lot of motion events per frame, these are
/// useless as the extra precision is wasted and these events tend to overflow our
/// otherwise very adequate event buffers.  So this function removes and compresses redundant
/// events.
fn compress_events(vec: &mut Vec<Event>, new_event: Event) {
    match new_event {
        Event::WindowEvent { ref event, .. } => match event {
            WindowEvent::CursorMoved { .. } => {
                let mut iter = vec.iter_mut();
                while let Some(stored_event) = iter.next_back() {
                    match stored_event {
                        Event::WindowEvent {
                            event: WindowEvent::CursorMoved { .. },
                            ..
                        } => {
                            mem::replace(stored_event, new_event.clone());
                            return;
                        }

                        Event::WindowEvent {
                            event: WindowEvent::AxisMotion { .. },
                            ..
                        } => {}

                        Event::DeviceEvent {
                            event: DeviceEvent::Motion { .. },
                            ..
                        } => {}

                        _ => {
                            break;
                        }
                    }
                }
            }

            WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => {
                let mut iter = vec.iter_mut();
                while let Some(stored_event) = iter.next_back() {
                    match stored_event {
                        Event::WindowEvent {
                            event:
                                WindowEvent::AxisMotion {
                                    axis: stored_axis,
                                    device_id: stored_device,
                                    value: ref mut stored_value,
                                },
                            ..
                        } => {
                            if device_id == stored_device && axis == stored_axis {
                                *stored_value += value;
                                return;
                            }
                        }

                        Event::WindowEvent {
                            event: WindowEvent::CursorMoved { .. },
                            ..
                        } => {}

                        Event::DeviceEvent {
                            event: DeviceEvent::Motion { .. },
                            ..
                        } => {}

                        _ => {
                            break;
                        }
                    }
                }
            }

            _ => {}
        },

        Event::DeviceEvent {
            device_id,
            event: DeviceEvent::Motion { axis, value },
        } => {
            let mut iter = vec.iter_mut();
            while let Some(stored_event) = iter.next_back() {
                match stored_event {
                    Event::DeviceEvent {
                        device_id: stored_device,
                        event:
                            DeviceEvent::Motion {
                                axis: stored_axis,
                                value: ref mut stored_value,
                            },
                    } => {
                        if device_id == *stored_device && axis == *stored_axis {
                            *stored_value += value;
                            return;
                        }
                    }

                    Event::WindowEvent {
                        event: WindowEvent::CursorMoved { .. },
                        ..
                    } => {}

                    Event::WindowEvent {
                        event: WindowEvent::AxisMotion { .. },
                        ..
                    } => {}

                    _ => {
                        break;
                    }
                }
            }
        }

        _ => {}
    }
    vec.push(new_event);
}