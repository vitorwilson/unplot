# Linux packaging notes

Which Linux artifact to use, and why the `.AppImage` needs a post-build fix.

## Which download should I use?

- **Debian / Ubuntu / Mint …** → the **`.deb`**. It depends on the distro's
  `libwebkit2gtk-4.1-0`, so it always uses your system's WebKitGTK and Mesa.
- **Fedora / openSUSE …** → the **`.rpm`**, for the same reason.
- **Arch / rolling-release / anything else** → the **`.AppImage`** works, or
  [run from source](../README.md#run-from-source) (`just dev`), which links your
  system WebKitGTK directly.

The `.deb`/`.rpm` are the most robust because they never bundle the browser
engine or the graphics stack — the OS keeps both up to date (WebKitGTK is a
security-critical component, so that is also the safer default).

## The AppImage `EGL_BAD_PARAMETER` crash

**Symptom.** On some systems the `.AppImage` aborts immediately, before any
window, with:

```
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
```

**Root cause.** Tauri builds the AppImage with `linuxdeploy-plugin-gtk`, which
bundles the _build machine's_ `libwayland-client/-cursor/-egl/-server` and
`libepoxy` and puts them ahead of the host's on the loader path. At startup
WebKitGTK's GPU process calls `eglGetPlatformDisplay`; the host's Mesa `libEGL`
(tied to the running kernel's DRM) is then handed the bundle's foreign-build
`libwayland`, the Wayland protocol handshake mismatches, and EGL fails. WebKitGTK
2.44+ has no non-EGL fallback, so it aborts. This is the well-known "**never
bundle the Mesa-coupled display libraries in an AppImage**" trap — those
libraries must come from the running system.

**Why env vars don't help.** `WEBKIT_DISABLE_DMABUF_RENDERER`,
`WEBKIT_DISABLE_COMPOSITING_MODE`, `LIBGL_ALWAYS_SOFTWARE`, forcing
`GDK_BACKEND` — none work. The abort happens while WebKit is _creating_ the base
EGL display, upstream of the renderer choice those variables control.

## The fix: `bin/unbundle-appimage-gl`

The release pipeline runs [`bin/unbundle-appimage-gl`](../bin/unbundle-appimage-gl)
on the freshly built `.AppImage`. It extracts the bundle, deletes the
host-coupled GL/display libraries (`libwayland-*`, `libepoxy`, and defensively
`libEGL`/`libGL`/`libgbm`/`libdrm` …) so the loader falls through to the host's,
and repacks the AppImage — reusing its own runtime header and matching its
squashfs compression. Everything else (WebKit, GTK, GStreamer) stays bundled.

It runs as a step in [`release.yml`](../.github/workflows/release.yml) after
`tauri-action`, replacing the uploaded asset. Swapping the bytes is safe here
because no updater artifact is signed (`createUpdaterArtifacts` is off).

**Patch an older download yourself.** If you have an `.AppImage` from before this
fix, repair it in place with the same script (needs `squashfs-tools`):

```sh
bin/unbundle-appimage-gl unplot_x.y.z_amd64.AppImage
```
