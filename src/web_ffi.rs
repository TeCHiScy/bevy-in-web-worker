use crate::{WorkerApp, bevy_app::init_app};
use bevy::prelude::*;
use js_sys::global;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::DedicatedWorkerGlobalScope;

static APP: OnceCell<u64> = OnceCell::new();

#[wasm_bindgen]
extern "C" {
    /// 在 app 初始化完成前，info! 无法使用，打印日志得用它
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
}

#[wasm_bindgen(raw_module = "./worker.js")]
extern "C" {
    /// 执行阻塞
    /// 由于 wasm 环境不支持 std::thread, 交由 js 环境代为执行
    ///
    /// 在 worker 环境执行
    pub(crate) fn block_from_worker();
}

fn set_onmessage() {
    let global = global().dyn_into::<DedicatedWorkerGlobalScope>().unwrap();
    let closure: Closure<dyn Fn(web_sys::MessageEvent)> = Closure::new(on_message);
    let func = closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
    global.set_onmessage(Some(&func));
    closure.forget();
}

fn post_message(val: &JsValue) {
    let global = global().dyn_into::<DedicatedWorkerGlobalScope>().unwrap();
    let _ = global.post_message(val);
}

#[wasm_bindgen]
pub async fn init_bevy_app(canvas: web_sys::OffscreenCanvas, scale_factor: f32) {
    let app = init_app(canvas, scale_factor);
    let _ = APP.set(Box::into_raw(Box::new(app)) as u64);

    // take over event loop
    set_onmessage();

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new({
        let app = wapp();
        move || {
            if app.window == Entity::PLACEHOLDER {
                if app.try_finish() {
                    let val = serde_wasm_bindgen::to_value(&Message {
                        ty: "ready".to_string(),
                        ..default()
                    })
                    .unwrap();
                    post_message(&val);
                }
                request_animation_frame(f.borrow().as_ref().unwrap());
                return;
            }

            block_from_worker();
            app.update();
            if app.should_exit().is_some() {
                app.close_window();
                return;
            }

            request_animation_frame(f.borrow().as_ref().unwrap());
        }
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
}

// https://wasm-bindgen.github.io/wasm-bindgen/examples/request-animation-frame.html
// https://developer.mozilla.org/en-US/docs/Web/API/DedicatedWorkerGlobalScope/requestAnimationFrame
fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    let _ = global()
        .dyn_into::<DedicatedWorkerGlobalScope>()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref());
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MouseEvent {
    alt_key: bool,
    button: i16,
    buttons: u16,
    client_x: f32,
    client_y: f32,
    ctrl_key: bool,
    meta_key: bool,
    offset_x: f32,
    offset_y: f32,
    page_x: f64,
    page_y: f64,
    screen_x: f64,
    screen_y: f64,
    shift_key: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardEvent {
    alt_key: bool,
    code: String,
    ctrl_key: bool,
    is_composing: bool,
    key: String,
    location: u32,
    meta_key: bool,
    shift_key: bool,
    repeat: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WheelEvent {
    delta_x: f32,
    delta_y: f32,
    delta_z: f32,
    delta_mode: u8,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Message {
    pub ty: String,
    pub event: Option<String>,
    pub list: Option<Vec<u64>>,
}

fn on_message(ev: web_sys::MessageEvent) {
    let data = ev.data();
    let msg: Message = serde_wasm_bindgen::from_value(data).unwrap_or_default();
    match msg.ty.as_str() {
        "mouse_up" | "mouse_down" | "mouse_move" => {
            if let Some(ev) = msg.event {
                on_mouse_event(msg.ty.as_str(), ev);
            }
        }
        "key_up" | "key_down" => {
            if let Some(ev) = msg.event {
                on_key_event(msg.ty.as_str(), ev);
            }
        }
        "wheel" => {
            if let Some(ev) = msg.event {
                on_wheel_event(msg.ty.as_str(), ev);
            }
        }
        _ => {}
    }

    /*
    case "blockRender":
      renderBlockTime = data.blockTime;
     */
}

fn on_mouse_event(r#type: &str, event: String) {
    let d = serde_json::from_str::<MouseEvent>(&event).unwrap_or_default();
    info!("[worker] <- {}: {:?}", r#type, &d);
    let app = wapp();
    match r#type {
        "mouse_up" => app.on_mouse_up(d.button),
        "mouse_down" => app.on_mouse_down(d.button, d.offset_x, d.offset_y),
        "mouse_move" => app.on_mouse_move(d.offset_x, d.offset_y),
        _ => {}
    }
}

fn on_key_event(r#type: &str, event: String) {
    let d = serde_json::from_str::<KeyboardEvent>(&event).unwrap_or_default();
    info!("[worker] <- {}: {:?}", r#type, &d);
    let app = wapp();
    match r#type {
        "key_up" => app.on_key_up(d.code, d.repeat),
        "key_down" => app.on_key_down(d.code, d.repeat),
        _ => {}
    }
}

fn on_wheel_event(r#type: &str, event: String) {
    let d = serde_json::from_str::<WheelEvent>(&event).unwrap_or_default();
    info!("[worker] <- {}: {:?}", r#type, &d);
    wapp().on_wheel(d.delta_x, d.delta_y, d.delta_mode);
}

fn wapp() -> &'static mut WorkerApp {
    let ptr = APP.get().copied().unwrap();
    unsafe { &mut *(ptr as *mut WorkerApp) }
}