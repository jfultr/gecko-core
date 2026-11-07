// Canonical code-viewer snippet for Gecko learning articles.
// `articles.py bundle` places it unchanged in one inline <script> after the
// Monaco library block; `articles.py check` compares it with this file, so any
// edit here must be reviewed as a security change (see gecko-article-public).
// Code stays readable without JavaScript: blocks are <pre><code data-lang="rust">.
// With the Monaco bundle present, each block becomes a read-only editor.
(function () {
  try {
    if (!window.monaco) return;
    var root = document.documentElement;
    var query = window.matchMedia("(prefers-color-scheme: dark)");
    function theme() {
      var dark = root.dataset.theme === "dark" || (root.dataset.theme !== "light" && query.matches);
      return dark ? "vs-dark" : "vs";
    }
    document.querySelectorAll("pre > code[data-lang]").forEach(function (code) {
      var pre = code.parentNode;
      var host = document.createElement("div");
      host.className = "code-host";
      pre.parentNode.insertBefore(host, pre);
      var editor = monaco.editor.create(host, {
        value: code.textContent.replace(/\n$/, ""),
        language: code.getAttribute("data-lang"),
        theme: theme(),
        readOnly: true,
        domReadOnly: true,
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        automaticLayout: true,
        folding: false,
        renderLineHighlight: "none",
        overviewRulerLanes: 0,
        hideCursorInOverviewRuler: true,
        contextmenu: false,
        fontSize: 13,
        lineHeight: 21,
        fontFamily: "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace",
        padding: { top: 12, bottom: 12 },
        scrollbar: { alwaysConsumeMouseWheel: false, vertical: "hidden", horizontalScrollbarSize: 8 }
      });
      function fit() {
        host.style.height = editor.getContentHeight() + "px";
        editor.layout();
      }
      editor.onDidContentSizeChange(fit);
      fit();
      pre.hidden = true;
    });
    query.addEventListener("change", function () {
      monaco.editor.setTheme(theme());
    });
  } catch (e) {
    /* leave the plain code blocks as they are */
  }
})();
