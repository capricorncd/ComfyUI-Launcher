import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const script = readFileSync(new URL('../src-tauri/src/launcher.js', import.meta.url), 'utf8');
for (const [origin, child, enabled] of [
    ['http://127.0.0.1:8188', false, true],
    ['http://127.0.0.1:8188', true, false],
    ['https://example.com', false, false],
    ['http://127.0.0.1:9999', false, false],
]) {
    const calls = [];
    const window = { __TAURI__: { core: { invoke: async command => { calls.push(command); return 'D:/project'; } } } };
    window.top = child ? {} : window;
    vm.runInNewContext(script, { window, location: { origin } });
    assert.equal(Boolean(window.__COMFYUI_LAUNCHER__), enabled);
    if (enabled) {
        const bridge = window.__COMFYUI_LAUNCHER__;
        assert.equal(bridge.apiVersion, 1);
        assert(Object.isFrozen(bridge));
        assert(Object.isFrozen(bridge.capabilities));
        assert.equal(await bridge.pickDirectory(), 'D:/project');
        assert.deepEqual(calls, ['launcher_pick_directory']);
    }
}
const capability = JSON.parse(readFileSync(new URL('../src-tauri/capabilities/timeline.json', import.meta.url)));
assert.deepEqual(capability.permissions, ['allow-launcher-pick-directory']);
assert.equal(capability.local, false);
assert.deepEqual(capability.windows, ['main']);
console.log('Launcher bridge: exact origin, top frame, immutable capabilities and native picker dispatch passed');
