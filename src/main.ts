import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type ComfyStatus =
  | { state: "stopped" }
  | { state: "starting" }
  | { state: "ready"; url: string }
  | { state: "failed"; message: string; logTail: string[] };

const spinnerEl = document.querySelector<HTMLDivElement>("#spinner")!;
const titleEl = document.querySelector<HTMLDivElement>("#status-title")!;
const logEl = document.querySelector<HTMLPreElement>("#log-tail")!;
const retryBtn = document.querySelector<HTMLButtonElement>("#retry-btn")!;

let liveLog: string[] = [];

function renderLog() {
  logEl.textContent = liveLog.slice(-300).join("\n");
  logEl.hidden = liveLog.length === 0;
  logEl.scrollTop = logEl.scrollHeight;
}

function render(status: ComfyStatus) {
  switch (status.state) {
    case "stopped":
      spinnerEl.hidden = false;
      titleEl.textContent = "等待启动...";
      titleEl.classList.remove("failed");
      retryBtn.hidden = true;
      break;
    case "starting":
      spinnerEl.hidden = false;
      titleEl.textContent = "正在启动 ComfyUI...";
      titleEl.classList.remove("failed");
      retryBtn.hidden = true;
      break;
    case "ready":
      spinnerEl.hidden = false;
      titleEl.textContent = "启动完成，正在打开界面...";
      titleEl.classList.remove("failed");
      retryBtn.hidden = true;
      break;
    case "failed":
      spinnerEl.hidden = true;
      titleEl.textContent = status.message;
      titleEl.classList.add("failed");
      liveLog = status.logTail;
      renderLog();
      retryBtn.hidden = false;
      break;
  }
}

retryBtn.addEventListener("click", () => {
  retryBtn.hidden = true;
  invoke("start_or_restart").catch((e) => {
    titleEl.textContent = `重试失败: ${e}`;
  });
});

window.addEventListener("DOMContentLoaded", async () => {
  await listen<ComfyStatus>("comfy-status", (event) => render(event.payload));
  await listen<string>("comfy-log", (event) => {
    liveLog.push(event.payload);
    renderLog();
  });

  const status = await invoke<ComfyStatus>("get_status");
  render(status);
});
