const worker = new Worker("./worker.js", { type: "module" });

worker.onmessage = async (event) => {
  let data = event.data;
  switch (data.ty) {
    case "ready":
      addEventObserver();
      let loading = document.getElementById("loading");
      loading.style.display = "none";
      break;
    case "pick":
      document.getElementById("hovers").innerText = data.list;
      break;
    default:
      break;
  }
};

function resizeCanvas(containerID) {
  let elem = document.getElementById(containerID);
  let canvas = elem.children[0];
  let ratio = window.devicePixelRatio;
  canvas.width = elem.clientWidth * ratio;
  canvas.height = elem.clientHeight * ratio;
  canvas.style.width = elem.clientWidth + "px";
  canvas.style.height = elem.clientHeight + "px";
  canvas.style.maxWidth = elem.clientWidth + "px";
  canvas.style.maxHeight = elem.clientHeight + "px";
}

function stringifyEvent(e) {
  const obj = {};
  for (let k in e) {
    obj[k] = e[k];
  }
  return JSON.stringify(
    obj,
    (k, v) => {
      if (v instanceof Node) return "Node";
      if (v instanceof Window) return "Window";
      return v;
    },
    " "
  );
}

// https://macroquad.rs/examples/
function addEventObserver() {
  let container = document.getElementById("worker-thread-container");
  container.onmousemove = function (event) {
    event.preventDefault();
    window.blockMS(window.mousemoveBlockTime);
    worker.postMessage({
      ty: "mouse_move",
      event: stringifyEvent(event),
    });
  };

  container.onwheel = function (event) {
    event.preventDefault();
    worker.postMessage({
      ty: "wheel",
      event: stringifyEvent(event),
    });
  };

  container.onkeyup = function (event) {
    if (!(event.key === "r" && event.metakey)) {
      event.preventDefault();
      worker.postMessage({
        ty: "key_up",
        event: stringifyEvent(event),
      });
    }
  };

  container.onkeydown = function (event) {
    if (!(event.key === "r" && event.metakey)) {
      event.preventDefault();
      worker.postMessage({
        ty: "key_down",
        event: stringifyEvent(event),
      });
    }
  };

  container.onmouseup = function (event) {
    worker.postMessage({
      ty: "mouse_up",
      event: stringifyEvent(event),
    });
  };

  container.onmousedown = function (event) {
    worker.postMessage({
      ty: "mouse_down",
      event: stringifyEvent(event),
    });
  };

  container.onclick = function (event) {
    event.preventDefault();
    container.focus();
  };

  container.onfocus = function (event) {
    container.style.border = "2px solid red";
  };

  container.onblur = function (event) {
    container.style.border = "2px solid black";
  };
}

window.blockWorkerRender = (dt) => {
  worker.postMessage({ ty: "blockRender", blockTime: dt });
};

if ("navigator" in window && "gpu" in navigator) {
  navigator.gpu
    .requestAdapter()
    .then((_adapter) => {
      resizeCanvas("worker-thread-container");
      // 创建渲染窗口
      let canvas = document.getElementById("worker-thread-canvas");
      let offscreenCanvas = canvas.transferControlToOffscreen();
      worker.postMessage(
        {
          ty: "start",
          canvas: offscreenCanvas,
          devicePixelRatio: window.devicePixelRatio,
        },
        [offscreenCanvas]
      );
    })
    .catch((_error) => {
      console.error(_error);
    });
} else {
  // 浏览器不支持 navigator.gpu
  alert("请使用 Chrome 或者 Edge 113+ 浏览器版本");
}
