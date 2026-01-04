use bevy::{app::PluginsState, ecs::system::SystemState, prelude::*, window::WindowCloseRequested};
use bevy_input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
    mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
};
use smol_str::SmolStr;
use std::ops::{Deref, DerefMut};

mod bevy_app;
mod canvas_plugin;
mod keyboard;
mod ray_pick;
mod web_ffi;

use keyboard::{AsKey, AsKeyCode};

pub(crate) use canvas_plugin::{OffscreenCanvas, OffscreenCanvasPlugin};

pub struct WorkerApp {
    pub app: App,
    /// 手动包装事件需要
    pub window: Entity,
    pub scale_factor: f32,
}

impl Deref for WorkerApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

impl DerefMut for WorkerApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.app
    }
}

impl WorkerApp {
    pub fn new(app: App, scale_factor: f32) -> Self {
        Self {
            app,
            window: Entity::PLACEHOLDER,
            scale_factor,
        }
    }

    pub fn to_physical_size(&self, x: f32, y: f32) -> Vec2 {
        Vec2::new(x * self.scale_factor, y * self.scale_factor)
    }

    pub fn try_finish(&mut self) -> bool {
        if self.plugins_state() != PluginsState::Ready {
            return false;
        }

        self.finish();
        self.cleanup();

        // 缓存 window 对象到 app 上
        let mut state: SystemState<Query<(Entity, &Window)>> =
            SystemState::from_world(self.world_mut());
        if let Ok((entity, _)) = state.get(self.world_mut()).single() {
            self.window = entity;
        }
        true
    }

    fn on_mouse_up(&mut self, button: i16) {
        let window = self.window;
        self.world_mut().write_message(MouseButtonInput {
            button: mouse_button(button),
            state: ButtonState::Released,
            window,
        });
    }

    fn on_mouse_down(&mut self, button: i16, x: f32, y: f32) {
        let window = self.window;
        self.world_mut().write_message(MouseButtonInput {
            button: mouse_button(button),
            state: ButtonState::Pressed,
            window,
        });
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        // 将逻辑像转换成物理像素
        let window = self.window;
        let position = self.to_physical_size(x, y);
        self.world_mut().write_message(CursorMoved {
            position,
            delta: None,
            window,
        });
    }

    fn on_key_up(&mut self, code: String, repeat: bool) {
        let window = self.window;
        let key = code.as_str().as_key();
        let text = key_text(&key);
        self.world_mut().write_message(KeyboardInput {
            key_code: code.as_str().as_key_code(),
            logical_key: key,
            state: ButtonState::Released,
            text: text,
            repeat,
            window,
        });
    }

    fn on_key_down(&mut self, code: String, repeat: bool) {
        let window = self.window;
        let key = code.as_str().as_key();
        let text = key_text(&key);
        self.world_mut().write_message(KeyboardInput {
            key_code: code.as_str().as_key_code(),
            logical_key: key,
            state: ButtonState::Pressed,
            text: text,
            repeat,
            window,
        });
    }

    fn on_wheel(&mut self, delta_x: f32, delta_y: f32, delta_mode: u8) {
        let window = self.window;
        self.world_mut().write_message(MouseWheel {
            x: delta_x,
            y: delta_y,
            unit: mouse_scroll_unit(delta_mode),
            window,
        });
    }

    fn close_window(&mut self) {
        let mut state: SystemState<Query<(Entity, &mut Window)>> =
            SystemState::from_world(self.world_mut());
        let windows = state.get_mut(self.world_mut());
        let (entity, _window) = windows.iter().last().unwrap();
        self.world_mut()
            .write_message(WindowCloseRequested { window: entity });
        state.apply(self.world_mut());

        self.update();
    }
}

fn mouse_button(button: i16) -> MouseButton {
    match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        _ => MouseButton::Other(button as u16),
    }
}

fn mouse_scroll_unit(delta_mode: u8) -> MouseScrollUnit {
    if delta_mode == 1 {
        MouseScrollUnit::Line
    } else {
        MouseScrollUnit::Pixel
    }
}

// https://github.com/rust-windowing/winit/blob/da6220060e7626c11332354cc26cd47e2937c200/winit-web/src/web_sys/event.rs#L265
pub fn key_text(key: &Key) -> Option<SmolStr> {
    match key {
        Key::Character(text) => Some(text.clone()),
        Key::Tab => Some(SmolStr::new("\t")),
        Key::Enter => Some(SmolStr::new("\r")),
        _ => None,
    }
    .map(SmolStr::new)
}
