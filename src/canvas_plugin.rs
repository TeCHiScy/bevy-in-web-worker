use bevy::{
    ecs::message::MessageWriter,
    prelude::*,
    window::{RawHandleWrapper, WindowClosed, WindowCreated, WindowWrapper, exit_on_all_closed},
};
use core::ptr::NonNull;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wasm_bindgen::JsValue;

unsafe impl Send for OffscreenCanvas {}
unsafe impl Sync for OffscreenCanvas {}

#[derive(Debug, Clone, Resource)]
pub struct OffscreenCanvas {
    inner: web_sys::OffscreenCanvas,
    scale_factor: f32,
}

impl OffscreenCanvas {
    pub const fn new(canvas: web_sys::OffscreenCanvas, scale_factor: f32) -> Self {
        Self {
            inner: canvas,
            scale_factor,
        }
    }

    pub fn physical_resolution(&self) -> (u32, u32) {
        (self.inner.width(), self.inner.height())
    }
}

impl HasWindowHandle for OffscreenCanvas {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::{RawWindowHandle, WebOffscreenCanvasWindowHandle, WindowHandle};

        let value: &JsValue = &self.inner;
        let obj: NonNull<core::ffi::c_void> = NonNull::from(value).cast();
        let handle = WebOffscreenCanvasWindowHandle::new(obj);
        let raw = RawWindowHandle::WebOffscreenCanvas(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw)) }
    }
}

impl HasDisplayHandle for OffscreenCanvas {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::{DisplayHandle, RawDisplayHandle, WebDisplayHandle};
        let handle = WebDisplayHandle::new();
        let raw = RawDisplayHandle::Web(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
    }
}

pub struct OffscreenCanvasPlugin;

impl Plugin for OffscreenCanvasPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Last,
            (
                changed_window.ambiguous_with(exit_on_all_closed),
                // Update the state of the window before attempting to despawn to ensure consistent event ordering
                despawn_window.after(changed_window),
            ),
        )
        .add_systems(Startup, spawn_window);
    }
}

pub fn spawn_window(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Window), Added<Window>>,
    mut writer: MessageWriter<WindowCreated>,
    canvas: ResMut<OffscreenCanvas>,
) {
    // entity -> window + canvas (raw handler wrapper / window wrapper)
    for (entity, mut window) in q.iter_mut() {
        let app_view = WindowWrapper::new(canvas.clone());
        let (logical_res, _scale_factor) = (app_view.physical_resolution(), app_view.scale_factor);

        // Update resolution of bevy window
        // I think scale is already handled in index.js by devicePixelRatio
        window.resolution.set_scale_factor(1.0);
        window
            .resolution
            .set(logical_res.0 as f32, logical_res.1 as f32);

        let raw_window_wrapper = RawHandleWrapper::new(&app_view);
        commands.entity(entity).insert(raw_window_wrapper.unwrap());
        writer.write(WindowCreated { window: entity });
        break;
    }
}

pub fn despawn_window(
    mut closed: RemovedComponents<Window>,
    window_entities: Query<&Window>,
    mut close_events: MessageWriter<WindowClosed>,
) {
    for entity in closed.read() {
        crate::web_ffi::log("Closing window {:?entity}");
        if !window_entities.contains(entity) {
            close_events.write(WindowClosed { window: entity });
        }
    }
}

pub fn changed_window(mut _changed_windows: Query<(Entity, &mut Window), Changed<Window>>) {
    // TODO:
}
