(() => {
    window.addEventListener("keydown", (event) => {
        if (event.ctrlKey && !event.altKey && !event.metaKey && !event.shiftKey && event.key.toLowerCase() === "p") {
            // Cancel browser printing without blocking application shortcuts.
            event.preventDefault();
        }
    }, { capture: true });
})();
