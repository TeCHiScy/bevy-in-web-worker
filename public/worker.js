// https://caniuse.com/?search=module+worker
// https://stackoverflow.com/questions/44118600/web-workers-how-to-import-modules

import init, { init_bevy_app } from "./bevy_bg.js";

let renderBlockTime = 1;

export function block_from_worker() {
  const start = performance.now();
  while (performance.now() - start < renderBlockTime) {}
}

onmessage = async (ev) => {
  let data = ev.data;
  if (data.ty === "start") {
    await init();
    await init_bevy_app(data.canvas, data.devicePixelRatio);
  }
};
