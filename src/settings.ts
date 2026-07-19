import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";

interface Config {
  rootPath: string;
}

const pathInput = document.querySelector<HTMLInputElement>("#root-path")!;
const browseBtn = document.querySelector<HTMLButtonElement>("#browse-btn")!;
const saveBtn = document.querySelector<HTMLButtonElement>("#save-btn")!;
const cancelBtn = document.querySelector<HTMLButtonElement>("#cancel-btn")!;
const errorBox = document.querySelector<HTMLDivElement>("#error-box")!;

function showError(message: string) {
  errorBox.textContent = message;
  errorBox.hidden = false;
}

function clearError() {
  errorBox.hidden = true;
  errorBox.textContent = "";
}

browseBtn.addEventListener("click", async () => {
  const dir = await open({ directory: true, defaultPath: pathInput.value || undefined });
  if (typeof dir === "string") {
    pathInput.value = dir;
    clearError();
  }
});

saveBtn.addEventListener("click", async () => {
  clearError();
  const rootPath = pathInput.value.trim();
  if (!rootPath) {
    showError("请填写 ComfyUI 安装根目录");
    return;
  }
  saveBtn.disabled = true;
  try {
    await invoke("save_config", { rootPath });
    invoke("start_or_restart").catch(() => {});
    await getCurrentWindow().close();
  } catch (e) {
    showError(String(e));
  } finally {
    saveBtn.disabled = false;
  }
});

cancelBtn.addEventListener("click", async () => {
  try {
    await getCurrentWindow().close();
  } catch (e) {
    showError(String(e));
  }
});

window.addEventListener("DOMContentLoaded", async () => {
  const config = await invoke<Config>("get_config");
  pathInput.value = config.rootPath;
});
