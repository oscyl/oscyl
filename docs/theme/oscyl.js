// The book's chrome in the website's terms (see oscyl.css).
(function () {
    // The name in the menu bar leads home to the website, as on the site.
    const title = document.querySelector(".menu-title");
    if (title) {
        const home = document.createElement("a");
        home.href = path_to_root + "../";
        home.innerHTML = '<img src="' + path_to_root + 'theme/favicon.svg" alt="">';
        home.append(title.textContent);
        title.replaceChildren(home);
    }

    // Two themes, light and dark: the others are the same two (oscyl.css).
    const dark = document.getElementById("mdbook-theme-coal");
    if (dark) dark.textContent = "Dark";

    // Previous and next name the chapter they lead to.
    const names = new Map();
    document.querySelectorAll("#mdbook-sidebar .chapter a[href]").forEach(function (a) {
        names.set(a.href, a.textContent.replace(/^\s*\d+(\.\d+)*\.\s*/, "").trim());
    });
    document.querySelectorAll(".nav-wrapper .mobile-nav-chapters").forEach(function (a) {
        const name = names.get(a.href);
        if (!name) return;
        const label = document.createElement("small");
        label.textContent = a.rel === "prev" ? "Previous" : "Next";
        const chapter = document.createElement("b");
        chapter.textContent = name;
        a.append(label, chapter);
        a.removeAttribute("aria-label");
        a.classList.add("named");
    });
})();
