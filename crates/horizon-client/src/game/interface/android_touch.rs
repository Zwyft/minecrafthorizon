// game-module-cfg: target_os = "android"

//! The phone's screen as a gamepad: the on-screen touch controls' fingers folded into the
//! controller state the game already reads, and the controls drawn over the HUD.
//!
//! In play a finger is the pad ([`pad_state`]); anywhere else, and on every screen, a tap is a
//! click where it landed, so the title screen, options and inventories stay reachable without a
//! mouse. The shortcut buttons (chat, settings, F3, F5) press the key they name.
use super::event_loop::EventLoopCtl;
use super::input_trace::InputOrigin;
use super::*;
use horizon_client::debug_keys::KeyAction;
use horizon_client::gamepad::PadState;
use horizon_client::hud::{HudAtlas, HudBatch};
use horizon_client::touch::TouchControls;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use winit::event::{DeviceId, ElementState, MouseButton, Touch, TouchPhase};

/// The process's touch controls. One window's fingers, like `android_runtime`'s input mode is
/// one for the process; the server thread's `Game` never reads them (it polls no pads).
static TOUCH: LazyLock<Mutex<TouchControls>> =
    LazyLock::new(|| Mutex::new(TouchControls::default()));

fn controls() -> MutexGuard<'static, TouchControls> {
    TOUCH.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The pad state the fingers hold this frame; `Pads::poll_pads` merges it with a controller's.
pub(super) fn pad_state() -> PadState {
    controls().poll_state()
}

/// Draws the visible controls over the HUD.
fn draw(batch: &mut HudBatch, atlas: &HudAtlas) {
    controls().draw(batch, atlas);
}

/// What the shortcut buttons asked for since the last frame.
#[derive(Default)]
struct Requests {
    chat: bool,
    settings: bool,
    debug: bool,
    perspective: bool,
}

fn take_requests() -> Requests {
    let mut touch = controls();
    let requests = Requests {
        chat: touch.chat_requested,
        settings: touch.settings_requested,
        debug: touch.debug_requested,
        perspective: touch.perspective_requested,
    };
    touch.chat_requested = false;
    touch.settings_requested = false;
    touch.debug_requested = false;
    touch.perspective_requested = false;
    requests
}

/// The device id the made-up mouse events carry: not a real pointing device.
fn touch_pointer() -> DeviceId {
    DeviceId::dummy()
}

impl Game<'_> {
    /// A finger on the window. In play the touch controls make it a pad; on a screen or a menu
    /// it is a left click where it landed, moved with the finger and released when it lifts.
    pub(super) fn touch_event(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        touch: Touch,
    ) {
        let menu = self.screen != Screen::Playing || self.pause || self.title.is_some();
        if menu {
            match touch.phase {
                TouchPhase::Started => {
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::CursorMoved {
                            device_id: touch_pointer(),
                            position: touch.location,
                        },
                        InputOrigin::User,
                    );
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::MouseInput {
                            device_id: touch_pointer(),
                            state: ElementState::Pressed,
                            button: MouseButton::Left,
                        },
                        InputOrigin::User,
                    );
                }
                TouchPhase::Moved => {
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::CursorMoved {
                            device_id: touch_pointer(),
                            position: touch.location,
                        },
                        InputOrigin::User,
                    );
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    self.handle_window_event(
                        event_loop,
                        id,
                        WindowEvent::MouseInput {
                            device_id: touch_pointer(),
                            state: ElementState::Released,
                            button: MouseButton::Left,
                        },
                        InputOrigin::User,
                    );
                }
            }
            return;
        }
        let size = self.window.as_ref().map_or([1, 1], |window| {
            let size = window.inner_size();
            [size.width.max(1), size.height.max(1)]
        });
        let point = [
            (touch.location.x / f64::from(size[0])) as f32,
            (touch.location.y / f64::from(size[1])) as f32,
        ];
        let turn = {
            let mut touch_controls = controls();
            touch_controls.viewport(size[0], size[1]);
            touch_controls.update(
                touch.id,
                point,
                touch.phase == TouchPhase::Started,
                matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled),
            )
        };
        if touch.phase == TouchPhase::Started {
            // A finger is the person using the phone: it takes input over from the keyboard.
            self.pad.keyboard_took_over = false;
            self.pad.in_use = true;
        }
        if turn[0] != 0.0 || turn[1] != 0.0 {
            // A look drag turns the view as a grabbed mouse's motion does.
            self.mouse.on_move(
                0.0,
                0.0,
                f64::from(turn[0]) * f64::from(size[0]),
                f64::from(turn[1]) * f64::from(size[1]),
                true,
                true,
            );
            self.last_input = std::time::Instant::now();
        }
    }

    /// The shortcut buttons since the last frame, as the keys each one names: T opens chat,
    /// Escape opens the pause screen (whose Options page holds the settings), F3 the debug
    /// overlay and F5 the next camera perspective.
    pub(super) fn touch_requests(&mut self) {
        let requests = take_requests();
        for (asked, key) in [
            (requests.chat, KeyCode::KeyT),
            (requests.settings, KeyCode::Escape),
        ] {
            if asked && let Err(e) = self.key_pressed(key, key) {
                log::warn!("Horizon: touch button {key:?}: {e}");
            }
        }
        if requests.debug {
            self.debug_key(KeyCode::F3, KeyAction::Press);
            self.debug_key(KeyCode::F3, KeyAction::Release);
        }
        if requests.perspective {
            self.cycle_camera();
        }
    }

    /// The on-screen controls over the HUD, while a world is played and the HUD is shown.
    pub(super) fn draw_touch_controls(&mut self, batch: &mut HudBatch) {
        if self.screen != Screen::Playing || self.ui.hide_gui || self.replay.playing() {
            return;
        }
        draw(batch, &self.gui.atlas);
    }
}
