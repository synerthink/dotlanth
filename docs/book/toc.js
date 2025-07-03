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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting-started/installation.html"><strong aria-hidden="true">1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting-started/quickstart.html"><strong aria-hidden="true">2.</strong> Quickstart</a></li><li class="chapter-item expanded "><a href="getting-started/development-setup.html"><strong aria-hidden="true">3.</strong> Development Setup</a></li><li class="chapter-item expanded affix "><li class="part-title">CLI Tools</li><li class="chapter-item expanded "><a href="cli/dotvm.html"><strong aria-hidden="true">4.</strong> DotVM CLI</a></li><li class="chapter-item expanded "><a href="cli/dotdb.html"><strong aria-hidden="true">5.</strong> DotDB CLI</a></li><li class="chapter-item expanded affix "><li class="part-title">DOTVM</li><li class="chapter-item expanded "><a href="dotvm/overview.html"><strong aria-hidden="true">6.</strong> Overview</a></li><li class="chapter-item expanded "><a href="dotvm/architecture/core.html"><strong aria-hidden="true">7.</strong> Architecture</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dotvm/architecture/vm-architectures.html"><strong aria-hidden="true">7.1.</strong> VM Architectures</a></li><li class="chapter-item expanded "><a href="dotvm/architecture/instruction-set.html"><strong aria-hidden="true">7.2.</strong> Instruction Set</a></li><li class="chapter-item expanded "><a href="dotvm/architecture/bytecode-format.html"><strong aria-hidden="true">7.3.</strong> Bytecode Format</a></li><li class="chapter-item expanded "><a href="dotvm/architecture/paracontracts.html"><strong aria-hidden="true">7.4.</strong> ParaContracts</a></li></ol></li><li class="chapter-item expanded "><a href="dotvm/usage/basic-operations.html"><strong aria-hidden="true">8.</strong> Usage</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dotvm/usage/transpilation.html"><strong aria-hidden="true">8.1.</strong> Transpilation</a></li><li class="chapter-item expanded "><a href="dotvm/usage/execution.html"><strong aria-hidden="true">8.2.</strong> Execution</a></li><li class="chapter-item expanded "><a href="dotvm/usage/advanced-features.html"><strong aria-hidden="true">8.3.</strong> Advanced Features</a></li></ol></li><li class="chapter-item expanded "><a href="dotvm/api/core.html"><strong aria-hidden="true">9.</strong> API Reference</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dotvm/api/runtime.html"><strong aria-hidden="true">9.1.</strong> Runtime API</a></li><li class="chapter-item expanded "><a href="dotvm/api/compiler.html"><strong aria-hidden="true">9.2.</strong> Compiler API</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">DOTDB</li><li class="chapter-item expanded "><a href="dotdb/overview.html"><strong aria-hidden="true">10.</strong> Overview</a></li><li class="chapter-item expanded "><a href="dotdb/architecture/storage-engine.html"><strong aria-hidden="true">11.</strong> Architecture</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dotdb/architecture/state-management.html"><strong aria-hidden="true">11.1.</strong> State Management</a></li><li class="chapter-item expanded "><a href="dotdb/architecture/document-collections.html"><strong aria-hidden="true">11.2.</strong> Document Collections</a></li></ol></li><li class="chapter-item expanded "><a href="dotdb/usage/basic-operations.html"><strong aria-hidden="true">12.</strong> Usage</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dotdb/usage/document-management.html"><strong aria-hidden="true">12.1.</strong> Document Management</a></li><li class="chapter-item expanded "><a href="dotdb/usage/advanced-features.html"><strong aria-hidden="true">12.2.</strong> Advanced Features</a></li></ol></li><li class="chapter-item expanded "><a href="dotdb/api/core.html"><strong aria-hidden="true">13.</strong> API Reference</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dotdb/api/storage.html"><strong aria-hidden="true">13.1.</strong> Storage API</a></li><li class="chapter-item expanded "><a href="dotdb/api/state.html"><strong aria-hidden="true">13.2.</strong> State API</a></li></ol></li><li class="chapter-item expanded "><li class="part-title">Guides</li><li class="chapter-item expanded "><a href="guides/deployment.html"><strong aria-hidden="true">14.</strong> Deployment</a></li><li class="chapter-item expanded "><a href="guides/troubleshooting.html"><strong aria-hidden="true">15.</strong> Troubleshooting</a></li><li class="chapter-item expanded affix "><li class="part-title">Contributing</li><li class="chapter-item expanded "><a href="contributing/guidelines.html"><strong aria-hidden="true">16.</strong> Guidelines</a></li><li class="chapter-item expanded "><a href="contributing/code-standards.html"><strong aria-hidden="true">17.</strong> Code Standards</a></li><li class="chapter-item expanded "><a href="contributing/development-workflow.html"><strong aria-hidden="true">18.</strong> Development Workflow</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
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
