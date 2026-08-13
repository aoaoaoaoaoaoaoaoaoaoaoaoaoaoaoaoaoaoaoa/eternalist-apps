# Local egui-winit Patch

This is the source of `egui-winit` 0.36.1 with one compatibility repair in
`src/dropped_file.rs`.

The vendored source remains available under its upstream MIT terms in
`LICENSE-MIT`.

The published crate implements `egui::DroppedFile::bytes` unconditionally.
That method is native-only in egui 0.36.1, so merely compiling `egui-winit`
for `wasm32-unknown-unknown` fails. The local repair gates `bytes` to native
targets and supplies the required `bytes_async` method on wasm. Winit does not
expose dropped-file bytes on the web, so the wasm method reports that fact.

The atelier intercepts browser file events before they reach egui-winit; it
does not offer file import. Remove this patch when a released egui-winit
contains the target-correct implementation.
