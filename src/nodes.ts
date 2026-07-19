import { invoke } from "@tauri-apps/api/core";

interface NodeInfo {
  name: string;
  hasGit: boolean;
  remoteUrl: string | null;
  version: string;
  lastUpdate: string | null;
}

interface ActionResult {
  success: boolean;
  message: string;
}

interface CloneResult {
  success: boolean;
  message: string;
  dirName: string;
}

interface UpdateCheck {
  name: string;
  remoteVersion: string | null;
  upToDate: boolean;
}

interface RowRefs {
  githubVersionTd: HTMLTableCellElement;
  updateBtn: HTMLButtonElement | null;
  status: HTMLSpanElement | null;
}

const urlInput = document.querySelector<HTMLInputElement>("#clone-url")!;
const cloneBtn = document.querySelector<HTMLButtonElement>("#clone-btn")!;
const refreshBtn = document.querySelector<HTMLButtonElement>("#refresh-btn")!;
const cloneHint = document.querySelector<HTMLDivElement>("#clone-hint")!;
const rowsEl = document.querySelector<HTMLTableSectionElement>("#node-rows")!;

let nodes: NodeInfo[] = [];
let rowRefs = new Map<string, RowRefs>();

function normalizeUrl(url: string): string {
  return url
    .trim()
    .toLowerCase()
    .replace(/^https?:\/\//, "")
    .replace(/\.git$/, "")
    .replace(/\/+$/, "");
}

function findExistingByUrl(url: string): NodeInfo | undefined {
  const target = normalizeUrl(url);
  if (!target) return undefined;
  return nodes.find((n) => n.remoteUrl && normalizeUrl(n.remoteUrl) === target);
}

function formatDate(iso: string | null): string {
  if (!iso) return "-";
  return iso.slice(0, 10);
}

function renderRows() {
  rowRefs = new Map();
  if (nodes.length === 0) {
    rowsEl.innerHTML = `<tr><td colspan="5">未找到自定义节点</td></tr>`;
    return;
  }
  rowsEl.innerHTML = "";
  for (const node of nodes) {
    const tr = document.createElement("tr");
    tr.dataset.name = node.name;

    const nameTd = document.createElement("td");
    nameTd.textContent = node.name;

    const versionTd = document.createElement("td");
    versionTd.textContent = node.version;

    const githubVersionTd = document.createElement("td");
    githubVersionTd.textContent = node.hasGit ? "检查中..." : "-";

    const dateTd = document.createElement("td");
    dateTd.textContent = formatDate(node.lastUpdate);

    const actionTd = document.createElement("td");
    let updateBtn: HTMLButtonElement | null = null;
    let status: HTMLSpanElement | null = null;
    if (node.hasGit) {
      updateBtn = document.createElement("button");
      updateBtn.textContent = "更新";
      status = document.createElement("span");
      status.className = "row-status";
      const btn = updateBtn;
      const statusEl = status;
      btn.addEventListener("click", async () => {
        btn.disabled = true;
        statusEl.textContent = "更新中...";
        statusEl.className = "row-status";
        try {
          const result = await invoke<ActionResult>("pull_node", { name: node.name });
          statusEl.textContent = result.message;
          statusEl.className = "row-status " + (result.success ? "ok" : "error");
          if (result.success) {
            await loadNodes();
          }
        } catch (e) {
          statusEl.textContent = String(e);
          statusEl.className = "row-status error";
        } finally {
          btn.disabled = false;
        }
      });
      actionTd.appendChild(btn);
      actionTd.appendChild(status);
    } else {
      actionTd.textContent = "(非 git 仓库)";
    }

    tr.append(nameTd, versionTd, githubVersionTd, dateTd, actionTd);
    rowsEl.appendChild(tr);
    rowRefs.set(node.name, { githubVersionTd, updateBtn, status });
  }
}

function applyUpdateChecks(checks: UpdateCheck[]) {
  for (const check of checks) {
    const refs = rowRefs.get(check.name);
    if (!refs) continue;
    refs.githubVersionTd.textContent = check.remoteVersion ?? "未知";
    if (check.upToDate && refs.updateBtn && refs.status) {
      refs.updateBtn.disabled = true;
      refs.status.textContent = "已是最新";
      refs.status.className = "row-status ok";
    }
  }
}

async function checkUpdates() {
  try {
    const checks = await invoke<UpdateCheck[]>("check_node_updates");
    applyUpdateChecks(checks);
  } catch {
    // best-effort background check; leave "检查中..." cells as-is on failure
  }
}

async function loadNodes() {
  rowsEl.innerHTML = `<tr><td colspan="5">加载中...</td></tr>`;
  try {
    nodes = await invoke<NodeInfo[]>("list_custom_nodes");
    renderRows();
    checkUpdates();
  } catch (e) {
    rowsEl.innerHTML = `<tr><td colspan="5">加载失败: ${String(e)}</td></tr>`;
  }
}

function updateCloneState() {
  const url = urlInput.value.trim();
  const existing = url ? findExistingByUrl(url) : undefined;

  if (!url) {
    cloneBtn.disabled = true;
    cloneHint.textContent = "";
    cloneHint.className = "hint";
    return;
  }

  if (existing) {
    cloneBtn.disabled = true;
    cloneHint.textContent = `此仓库已存在于「${existing.name}」，请在下方列表中点击更新`;
    cloneHint.className = "hint warn";
    highlightRow(existing.name);
  } else {
    cloneBtn.disabled = false;
    cloneHint.textContent = "";
    cloneHint.className = "hint";
    clearHighlight();
  }
}

function highlightRow(name: string) {
  clearHighlight();
  const row = rowsEl.querySelector<HTMLTableRowElement>(`tr[data-name="${CSS.escape(name)}"]`);
  row?.classList.add("highlight");
  row?.scrollIntoView({ block: "center" });
}

function clearHighlight() {
  rowsEl.querySelectorAll("tr.highlight").forEach((el) => el.classList.remove("highlight"));
}

urlInput.addEventListener("input", updateCloneState);

cloneBtn.addEventListener("click", async () => {
  const url = urlInput.value.trim();
  if (!url) return;
  cloneBtn.disabled = true;
  cloneHint.textContent = "克隆中...";
  cloneHint.className = "hint";
  try {
    const result = await invoke<CloneResult>("clone_node", { url });
    cloneHint.textContent = result.message;
    cloneHint.className = "hint " + (result.success ? "" : "warn");
    if (result.success) {
      urlInput.value = "";
      await loadNodes();
    }
  } catch (e) {
    cloneHint.textContent = String(e);
    cloneHint.className = "hint warn";
  } finally {
    updateCloneState();
  }
});

refreshBtn.addEventListener("click", () => loadNodes());

window.addEventListener("DOMContentLoaded", () => {
  loadNodes();
});
