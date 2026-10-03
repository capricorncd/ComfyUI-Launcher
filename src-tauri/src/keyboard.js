(() => {
    window.addEventListener("keydown", (event) => {
        if (event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey && event.key.toLowerCase() === "p") {
            event.preventDefault();
            event.stopImmediatePropagation();
        }
    }, { capture: true });
})();
