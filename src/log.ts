import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const NEAR_BOTTOM_PX = 40;
const MAX_LINES = 2000;

const viewerEl = document.querySelector<HTMLPreElement>("#log-viewer")!;
const refreshBtn = document.querySelector<HTMLButtonElement>("#refresh-btn")!;

let lines: string[] = [];

function isNearBottom(): boolean {
  return viewerEl.scrollHeight - viewerEl.scrollTop - viewerEl.clientHeight < NEAR_BOTTOM_PX;
}

function render(stickToBottom: boolean) {
  viewerEl.textContent = lines.length > 0 ? lines.join("\n") : "(暂无日志)";
  if (stickToBottom) {
    viewerEl.scrollTop = viewerEl.scrollHeight;
  }
}

async function loadTail() {
  try {
    lines = await invoke<string[]>("get_log_tail");
    render(true);
  } catch (e) {
    viewerEl.textContent = `加载日志失败: ${String(e)}`;
  }
}

refreshBtn.addEventListener("click", () => loadTail());

window.addEventListener("DOMContentLoaded", async () => {
  await loadTail();

  await listen<string>("comfy-log", (event) => {
    const stick = isNearBottom();
    lines.push(event.payload);
    if (lines.length > MAX_LINES) {
      lines = lines.slice(-MAX_LINES);
    }
    render(stick);
  });
});
