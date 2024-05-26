use super::{Action, Map, PressedKey};
use crate::ui::utils::mul2s;
use ahash::AHashSet as HashSet;
use dust_core::emu::input::Keys as EmuKeys;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{KeyEvent, MouseButton, WindowEvent},
};
use sdl2::event::Event;

pub struct State {
    pressed_keys: HashSet<PressedKey>,
    touchscreen_center: LogicalPosition<f64>,
    touchscreen_size: LogicalSize<f64>,
    touchscreen_half_size: LogicalSize<f64>,
    touchscreen_rot: (f64, f64),
    touchscreen_rot_center: LogicalPosition<f64>,
    mouse_pos: LogicalPosition<f64>,
    touch_pos: Option<[u16; 2]>,
    prev_touch_pos: Option<[u16; 2]>,
    pressed_emu_keys: EmuKeys,
    pressed_hotkeys: HashSet<Action>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Changes {
    pub pressed: EmuKeys,
    pub released: EmuKeys,
    pub touch_pos: Option<Option<[u16; 2]>>,
}

impl State {
    pub fn new() -> Self {
        State {
            pressed_keys: HashSet::new(),
            touchscreen_size: Default::default(),
            touchscreen_center: Default::default(),
            touchscreen_half_size: Default::default(),
            touchscreen_rot: (0.0, 1.0),
            touchscreen_rot_center: Default::default(),
            mouse_pos: Default::default(),
            touch_pos: None,
            prev_touch_pos: None,
            pressed_emu_keys: EmuKeys::empty(),
            pressed_hotkeys: HashSet::new(),
        }
    }

    pub fn set_touchscreen_bounds_from_points(
        &mut self,
        center: [f32; 2],
        points: &[[f32; 2]; 4],
        rot: f32,
    ) {
        fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
            let x = b[0] - a[0];
            let y = b[1] - a[1];
            (x * x + y * y).sqrt()
        }

        let size = [
            distance(points[0], points[1]),
            distance(points[1], points[2]) * 0.5,
        ];
        self.set_touchscreen_bounds(
            center.into(),
            (center[0], center[1] + size[1] * 0.5).into(),
            size.into(),
            rot as f64,
        );
    }

    pub fn set_touchscreen_bounds(
        &mut self,
        rot_center: LogicalPosition<f64>,
        center: LogicalPosition<f64>,
        size: LogicalSize<f64>,
        rot: f64,
    ) {
        self.touchscreen_center = center;
        self.touchscreen_size = size;
        self.touchscreen_half_size = (size.width * 0.5, size.height * 0.5).into();
        self.touchscreen_rot = rot.sin_cos();
        self.touchscreen_rot_center = rot_center;
    }

    fn recalculate_touch_pos<const CLAMP: bool>(&mut self) {
        let mut diff = [
            self.mouse_pos.x - self.touchscreen_rot_center.x,
            self.mouse_pos.y - self.touchscreen_rot_center.y,
        ];
        diff = [
            self.touchscreen_rot_center.x
                + diff[0] * self.touchscreen_rot.1
                + diff[1] * self.touchscreen_rot.0
                - self.touchscreen_center.x,
            self.touchscreen_rot_center.y - diff[0] * self.touchscreen_rot.0
                + diff[1] * self.touchscreen_rot.1
                - self.touchscreen_center.y,
        ];
        if CLAMP {
            let scale = (self.touchscreen_half_size.width / diff[0])
                .abs()
                .min((self.touchscreen_half_size.height / diff[1]).abs())
                .min(1.0);
            diff = mul2s(diff, scale);
        } else if diff[0].abs() >= self.touchscreen_half_size.width
            || diff[1].abs() >= self.touchscreen_half_size.height
        {
            return;
        }
        self.touch_pos = Some([
            ((diff[0] / self.touchscreen_half_size.width + 1.0) * 2048.0).clamp(0.0, 4095.0) as u16,
            ((diff[1] / self.touchscreen_half_size.height + 1.0) * 1536.0).clamp(0.0, 3072.0)
                as u16,
        ]);
    }

    pub fn process_event(
        &mut self,
        event: &Event,
        scale_factor: f64,
        catch_new: bool,
    ) {
        // TODO
        // if let Event::WindowEvent { event, .. } = event {
        //     match event {
        //         WindowEvent::KeyboardInput {
        //             event:
        //                 KeyEvent {
        //                     physical_key,
        //                     state,
        //                     ..
        //                 },
        //             ..
        //         } => {
        //             let Ok(key) = (*physical_key).try_into() else {
        //                 return;
        //             };
        //             if state.is_pressed() {
        //                 if catch_new {
        //                     self.pressed_keys.insert(key);
        //                 }
        //             } else {
        //                 self.pressed_keys.remove(&key);
        //             }
        //         }
        //
        //         WindowEvent::CursorMoved { position, .. } => {
        //             self.mouse_pos = position.to_logical(scale_factor);
        //             if self.touch_pos.is_some() {
        //                 self.recalculate_touch_pos::<true>();
        //             }
        //         }
        //
        //         WindowEvent::MouseInput {
        //             state,
        //             button: MouseButton::Left,
        //             ..
        //         } => {
        //             if state.is_pressed() {
        //                 if catch_new {
        //                     self.recalculate_touch_pos::<false>();
        //                 }
        //             } else {
        //                 self.touch_pos = None;
        //             }
        //         }
        //
        //         WindowEvent::Focused(false) => {
        //             self.pressed_keys.clear();
        //             self.touch_pos = None;
        //         }
        //
        //         _ => {}
        //     }
        // }
    }

    pub fn drain_changes(
        &mut self,
        map: &Map,
        emu_playing: bool,
    ) -> (Vec<Action>, Option<Changes>) {
        let mut actions = Vec::new();
        for (&action, trigger) in &map.hotkeys {
            if let Some(trigger) = trigger {
                if trigger.activated(&self.pressed_keys) {
                    if self.pressed_hotkeys.insert(action) {
                        actions.push(action);
                    }
                } else {
                    self.pressed_hotkeys.remove(&action);
                }
            }
        }

        if !emu_playing {
            return (actions, None);
        }

        let mut new_pressed_emu_keys = EmuKeys::empty();
        for (&emu_key, trigger) in &map.keypad {
            if let Some(trigger) = trigger {
                new_pressed_emu_keys.set(emu_key, trigger.activated(&self.pressed_keys));
            }
        }

        let pressed = new_pressed_emu_keys & !self.pressed_emu_keys;
        let released = self.pressed_emu_keys & !new_pressed_emu_keys;
        let touch_pos = if self.touch_pos == self.prev_touch_pos {
            None
        } else {
            Some(self.touch_pos)
        };

        (
            actions,
            if touch_pos.is_some() || new_pressed_emu_keys != self.pressed_emu_keys {
                self.pressed_emu_keys = new_pressed_emu_keys;
                self.prev_touch_pos = self.touch_pos;
                Some(Changes {
                    pressed,
                    released,
                    touch_pos,
                })
            } else {
                None
            },
        )
    }
}
