import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const script = readFileSync(new URL('../src-tauri/src/keyboard.js', import.meta.url), 'utf8');
let handler;
vm.runInNewContext(script, {
    window: {
        addEventListener(type, callback, options) {
            assert.equal(type, 'keydown');
            assert.equal(options.capture, true);
            handler = callback;
        },
    },
});
for (const [keys, blocked] of [
    [{ key: 'p', ctrlKey: true }, true],
    [{ key: 'P', ctrlKey: true }, true],
    [{ key: 'p', ctrlKey: true, repeat: true }, true],
    [{ key: 'p' }, false],
    [{ key: 's', ctrlKey: true }, false],
    [{ key: 'c', ctrlKey: true }, false],
    [{ key: 'p', ctrlKey: true, shiftKey: true }, false],
    [{ key: 'p', ctrlKey: true, altKey: true }, false],
    [{ key: 'p', metaKey: true }, false],
]) {
    let prevented = false;
    let stopped = false;
    handler({
        ...keys,
        preventDefault() { prevented = true; },
        stopPropagation() { stopped = true; },
        stopImmediatePropagation() { stopped = true; },
    });
    assert.equal(prevented, blocked, JSON.stringify(keys));
    assert.equal(stopped, false, JSON.stringify(keys));
}
for (const file of ['lib.rs', 'windows.rs']) {
    const source = readFileSync(new URL(`../src-tauri/src/${file}`, import.meta.url), 'utf8');
    assert(source.includes('.initialization_script(include_str!("keyboard.js"))'));
}
console.log('Ctrl+P printing prevented; application shortcut propagation preserved');
