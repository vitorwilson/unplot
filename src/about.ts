// The About dialog: the app version, and links to the project's repository
// (license + documentation) and its issue tracker (report bugs). Both links open
// in the system browser through an injected opener — never by navigating the
// app's own webview away from the canvas.

/** The project's GitHub repository — its license and documentation live here. */
export const REPO_URL = "https://github.com/vitorwilson/unplot";

/** The issue tracker, for bug reports. */
export const ISSUES_URL = `${REPO_URL}/issues`;

/** The version line shown in the dialog, e.g. `v0.1.0`. Pure — unit-tested. */
export function versionLabel(version: string): string {
  return `v${version}`;
}

/** The dialog's DOM elements, injected so the view has no id coupling. */
export interface AboutElements {
  openButton: HTMLButtonElement;
  dialog: HTMLDialogElement;
  version: HTMLElement;
  closeButton: HTMLButtonElement;
  // The external links (repo, issues); each opens its own `href`.
  links: HTMLAnchorElement[];
}

/** What the view needs from the shell: the app version and a system-browser
 * opener. Injected so the DOM wiring stays free of Tauri imports. */
export interface AboutDeps {
  appVersion: () => Promise<string>;
  openExternal: (url: string) => Promise<void>;
}

/** Wire the About button: open the modal (filling in the version), close it, and
 * route every link to the system browser instead of navigating the webview. */
export function installAboutView(el: AboutElements, deps: AboutDeps): void {
  el.openButton.addEventListener("click", () => {
    void deps
      .appVersion()
      .then((version) => {
        el.version.textContent = versionLabel(version);
      })
      .catch(() => {
        el.version.textContent = "";
      });
    el.dialog.showModal();
  });

  el.closeButton.addEventListener("click", () => el.dialog.close());

  for (const link of el.links) {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      // Swallow opener failures (e.g. a URL outside the capability allowlist) so
      // they don't surface as an unhandled rejection.
      void deps.openExternal(link.href).catch(() => {});
    });
  }
}
