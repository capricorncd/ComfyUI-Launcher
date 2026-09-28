(() => {
    if (window.top !== window || location.origin !== "http://127.0.0.1:8188") return;
    Object.defineProperty(window, "__COMFYUI_LAUNCHER__", {
        value: Object.freeze({
            apiVersion: 1,
            capabilities: Object.freeze({ projectDirectory: true }),
            pickDirectory: () => window.__TAURI__.core.invoke("launcher_pick_directory"),
        }),
        writable: false,
        configurable: false,
    });
})();
