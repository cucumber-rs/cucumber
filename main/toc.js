// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="introduction.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="quickstart.html"><strong aria-hidden="true">2.</strong> Quickstart</a></li><li class="chapter-item expanded "><a href="writing/index.html"><strong aria-hidden="true">3.</strong> Writing tests</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="writing/capturing.html"><strong aria-hidden="true">3.1.</strong> Capturing and variation</a></li><li class="chapter-item expanded "><a href="writing/asserting.html"><strong aria-hidden="true">3.2.</strong> Asserting</a></li><li class="chapter-item expanded "><a href="writing/data_tables.html"><strong aria-hidden="true">3.3.</strong> Data tables</a></li><li class="chapter-item expanded "><a href="writing/doc_strings.html"><strong aria-hidden="true">3.4.</strong> Doc strings</a></li><li class="chapter-item expanded "><a href="writing/rule.html"><strong aria-hidden="true">3.5.</strong> Rule keyword</a></li><li class="chapter-item expanded "><a href="writing/background.html"><strong aria-hidden="true">3.6.</strong> Background keyword</a></li><li class="chapter-item expanded "><a href="writing/scenario_outline.html"><strong aria-hidden="true">3.7.</strong> Scenario Outline keyword</a></li><li class="chapter-item expanded "><a href="writing/hooks.html"><strong aria-hidden="true">3.8.</strong> Scenario hooks</a></li><li class="chapter-item expanded "><a href="writing/languages.html"><strong aria-hidden="true">3.9.</strong> Spoken languages</a></li><li class="chapter-item expanded "><a href="writing/tags.html"><strong aria-hidden="true">3.10.</strong> Tags</a></li><li class="chapter-item expanded "><a href="writing/retries.html"><strong aria-hidden="true">3.11.</strong> Retrying failed scenarios</a></li><li class="chapter-item expanded "><a href="writing/modules.html"><strong aria-hidden="true">3.12.</strong> Modules organization</a></li></ol></li><li class="chapter-item expanded "><a href="cli.html"><strong aria-hidden="true">4.</strong> CLI (command-line interface)</a></li><li class="chapter-item expanded "><a href="output/index.html"><strong aria-hidden="true">5.</strong> Output</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="output/terminal.html"><strong aria-hidden="true">5.1.</strong> Terminal</a></li><li class="chapter-item expanded "><a href="output/junit.html"><strong aria-hidden="true">5.2.</strong> JUnit XML report</a></li><li class="chapter-item expanded "><a href="output/json.html"><strong aria-hidden="true">5.3.</strong> Cucumber JSON format</a></li><li class="chapter-item expanded "><a href="output/multiple.html"><strong aria-hidden="true">5.4.</strong> Multiple outputs</a></li><li class="chapter-item expanded "><a href="output/tracing.html"><strong aria-hidden="true">5.5.</strong> tracing integration</a></li><li class="chapter-item expanded "><a href="output/intellij.html"><strong aria-hidden="true">5.6.</strong> IntelliJ Rust (libtest) integration</a></li></ol></li><li class="chapter-item expanded "><a href="architecture/index.html"><strong aria-hidden="true">6.</strong> Architecture</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="architecture/parser.html"><strong aria-hidden="true">6.1.</strong> Custom Parser</a></li><li class="chapter-item expanded "><a href="architecture/runner.html"><strong aria-hidden="true">6.2.</strong> Custom Runner</a></li><li class="chapter-item expanded "><a href="architecture/writer.html"><strong aria-hidden="true">6.3.</strong> Custom Writer</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString();
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
