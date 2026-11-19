// One theme choice for every Oscyl page: localStorage "oscyl-theme" is
// "light" or "dark", or absent to follow the system. The website and the API's
// front page use it here; the book (docs/theme/head.hbs) and the API pages
// (docs/theme/rustdoc/header.html) read it and write it from their own menus.
// Loaded in <head>, before the page draws, so it never flashes the other theme.
(function () {
    const root = document.documentElement;
    let theme = null;
    try { theme = localStorage.getItem("oscyl-theme"); } catch (e) { /* storage off: follow the system */ }
    if (theme !== "light" && theme !== "dark") theme = null;

    // Pictures with a dark version follow the chosen theme, not only the system's.
    function pictures() {
        document.querySelectorAll("source[data-dark]").forEach(function (source) {
            source.media = theme === "dark" ? "all" : theme === "light" ? "not all" : "(prefers-color-scheme: dark)";
        });
    }
    function apply(next) {
        theme = next;
        if (theme) root.dataset.theme = theme; else delete root.dataset.theme;
        pictures();
    }
    apply(theme);

    document.addEventListener("DOMContentLoaded", function () {
        pictures();
        const group = document.querySelector(".theme");
        if (!group) return;
        const buttons = group.querySelectorAll("button");
        function show() {
            buttons.forEach(function (b) { b.setAttribute("aria-pressed", String((b.value || null) === theme)); });
        }
        buttons.forEach(function (b) {
            b.addEventListener("click", function () {
                apply(b.value || null);
                try {
                    if (theme) localStorage.setItem("oscyl-theme", theme); else localStorage.removeItem("oscyl-theme");
                } catch (e) { /* the choice lasts for this page only */ }
                show();
            });
        });
        show();
        group.hidden = false;
    });
})();
