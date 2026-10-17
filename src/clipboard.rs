//! Image publication to the native clipboard, off the event loop.
//!
//! An X11 clipboard is served by its owner whenever another client pastes.
//! arboard, and egui-winit through it, answers every request with a single
//! `ChangeProperty`, so a PNG larger than the server's maximum request (16 MiB
//! with BIG-REQUESTS) silently pastes nothing anywhere. This owner sends such
//! payloads through ICCCM INCR transfers instead. macOS and Windows store the
//! image in the system pasteboard through arboard.

#![deny(missing_docs)]

use std::thread;

use anyhow::{Context as _, Result, anyhow, ensure};
use crossbeam_channel::{Receiver, TryRecvError, bounded};

use crate::NativeWake;

/// One 8-bit, unpremultiplied RGBA image.
pub struct RgbaImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl RgbaImage {
    /// Admit `rgba` as `width` × `height` rows of four bytes per pixel.
    pub fn new(width: usize, height: usize, rgba: Vec<u8>) -> Result<Self> {
        let expected = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|&bytes| bytes > 0);
        ensure!(
            expected == Some(rgba.len()),
            "{width}×{height} RGBA cannot occupy {} bytes",
            rgba.len()
        );
        Ok(Self {
            width: u32::try_from(width).context("image width exceeds PNG's range")?,
            height: u32::try_from(height).context("image height exceeds PNG's range")?,
            rgba,
        })
    }
}

/// One image's seizure of the system clipboard, settling on a worker thread.
///
/// The worker encodes the image and takes the clipboard, then wakes the host.
/// On X11 it remains the clipboard's owner, serving pastes until another client
/// takes the clipboard.
#[derive(Debug)]
#[must_use = "a seizure reports whether the clipboard holds the image"]
pub struct Seizure {
    outcome: Receiver<Result<()>>,
}

impl Seizure {
    /// Seize the system clipboard with `image`.
    pub fn image(image: RgbaImage, wake: NativeWake) -> Self {
        let (settle, outcome) = bounded(1);
        let worker = settle.clone();
        let spawned = thread::Builder::new()
            .name("eternalist-clipboard".to_owned())
            .spawn(move || {
                platform::seize(image, |seized| {
                    let _sent = worker.send(seized);
                    let _woken = wake.request_repaint();
                });
            });
        if let Err(error) = spawned {
            let _sent = settle.send(Err(error).context("spawn the clipboard worker"));
        }
        Self { outcome }
    }

    /// The outcome once the clipboard holds the image or the seizure failed;
    /// a pending seizure is returned unchanged.
    pub fn try_settle(self) -> std::result::Result<Result<()>, Self> {
        match self.outcome.try_recv() {
            Ok(seized) => Ok(seized),
            Err(TryRecvError::Empty) => Err(self),
            Err(TryRecvError::Disconnected) => Ok(Err(anyhow!(
                "the clipboard worker stopped without an outcome"
            ))),
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::time::{Duration, Instant};

    use anyhow::{Context as _, Result, ensure};
    use x11rb::{
        COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, CURRENT_TIME, NONE,
        connection::{Connection as _, RequestConnection as _},
        protocol::{
            Event,
            xproto::{
                Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, CreateWindowAux,
                EventMask, PropMode, Property, SELECTION_NOTIFY_EVENT, SelectionNotifyEvent,
                SelectionRequestEvent, Timestamp, Window, WindowClass,
            },
        },
        rust_connection::RustConnection,
        wrapper::ConnectionExt as _,
    };

    use super::RgbaImage;

    /// An abandoned INCR transfer is forgotten after this much silence.
    const STALE_TRANSFER: Duration = Duration::from_mins(1);
    /// Request bytes reserved for the `ChangeProperty` header.
    const HEADER: usize = 64;

    x11rb::atom_manager! {
        Atoms: AtomsCookie {
            CLIPBOARD,
            TARGETS,
            TIMESTAMP,
            INCR,
            PNG: b"image/png",
            STAMP: b"_ETERNALIST_CLIPBOARD_STAMP",
        }
    }

    pub(super) fn seize(image: RgbaImage, settle: impl FnOnce(Result<()>)) {
        match encode(image).and_then(Owner::seize) {
            Ok(owner) => {
                settle(Ok(()));
                if let Err(error) = owner.serve() {
                    tracing::warn!("clipboard owner stopped: {error:#}");
                }
            }
            Err(error) => settle(Err(error)),
        }
    }

    /// Encode and release the raw pixels: only the PNG outlives the seizure.
    fn encode(image: RgbaImage) -> Result<Vec<u8>> {
        let RgbaImage {
            width,
            height,
            rgba,
        } = image;
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().context("begin the clipboard PNG")?;
        writer
            .write_image_data(&rgba)
            .context("encode the clipboard PNG")?;
        writer.finish().context("finish the clipboard PNG")?;
        Ok(png)
    }

    /// One INCR transfer: the requestor deletes its property to ask for the
    /// next chunk, and a zero-length chunk ends the transfer.
    struct Transfer {
        requestor: Window,
        property: Atom,
        sent: usize,
        touched: Instant,
    }

    struct Owner {
        connection: RustConnection,
        window: Window,
        atoms: Atoms,
        acquired: Timestamp,
        png: Vec<u8>,
        chunk: usize,
        transfers: Vec<Transfer>,
    }

    impl Owner {
        fn seize(png: Vec<u8>) -> Result<Self> {
            let (connection, screen) =
                x11rb::connect(None).context("connect to the X11 clipboard")?;
            let root = connection.setup().roots[screen].root;
            let window = connection.generate_id()?;
            connection
                .create_window(
                    COPY_DEPTH_FROM_PARENT,
                    window,
                    root,
                    0,
                    0,
                    1,
                    1,
                    0,
                    WindowClass::INPUT_ONLY,
                    COPY_FROM_PARENT,
                    &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                )?
                .check()
                .context("create the clipboard window")?;
            let atoms = Atoms::new(&connection)?.reply()?;
            // ICCCM forbids seizing a selection at CurrentTime; a zero-length
            // append to our own window reports the server's present time.
            connection
                .change_property8(PropMode::APPEND, window, atoms.STAMP, AtomEnum::STRING, &[])?
                .check()
                .context("stamp the clipboard window")?;
            let acquired = loop {
                if let Event::PropertyNotify(notify) = connection.wait_for_event()?
                    && notify.window == window
                {
                    break notify.time;
                }
            };
            connection
                .set_selection_owner(window, atoms.CLIPBOARD, acquired)?
                .check()
                .context("seize the clipboard")?;
            ensure!(
                connection
                    .get_selection_owner(atoms.CLIPBOARD)?
                    .reply()?
                    .owner
                    == window,
                "another client kept the clipboard"
            );
            let chunk = connection.maximum_request_bytes().saturating_sub(HEADER);
            Ok(Self {
                connection,
                window,
                atoms,
                acquired,
                png,
                chunk,
                transfers: Vec::new(),
            })
        }

        /// Answer pastes until another client takes the clipboard. Transfers
        /// still in flight at that moment are abandoned.
        fn serve(mut self) -> Result<()> {
            loop {
                match self.connection.wait_for_event()? {
                    Event::SelectionClear(clear) if clear.selection == self.atoms.CLIPBOARD => {
                        return Ok(());
                    }
                    Event::SelectionRequest(request) => self.answer(&request)?,
                    Event::PropertyNotify(notify) if notify.state == Property::DELETE => {
                        self.advance(notify.window, notify.atom)?;
                    }
                    _ => {}
                }
                let now = Instant::now();
                self.transfers
                    .retain(|transfer| now.duration_since(transfer.touched) < STALE_TRANSFER);
                self.connection.flush()?;
            }
        }

        fn answer(&mut self, request: &SelectionRequestEvent) -> Result<()> {
            // Obsolete requestors name no property and expect the target's.
            let property = if request.property == NONE {
                request.target
            } else {
                request.property
            };
            let admitted = request.owner == self.window
                && request.selection == self.atoms.CLIPBOARD
                && (request.time == CURRENT_TIME || request.time >= self.acquired);
            let supplied = admitted && self.supply(request.requestor, request.target, property)?;
            self.connection
                .send_event(
                    false,
                    request.requestor,
                    EventMask::NO_EVENT,
                    SelectionNotifyEvent {
                        response_type: SELECTION_NOTIFY_EVENT,
                        sequence: 0,
                        time: request.time,
                        requestor: request.requestor,
                        selection: request.selection,
                        target: request.target,
                        property: if supplied { property } else { NONE },
                    },
                )?
                .ignore_error();
            Ok(())
        }

        fn supply(&mut self, requestor: Window, target: Atom, property: Atom) -> Result<bool> {
            let atoms = self.atoms;
            if target == atoms.TARGETS {
                self.connection
                    .change_property32(
                        PropMode::REPLACE,
                        requestor,
                        property,
                        AtomEnum::ATOM,
                        &[atoms.TARGETS, atoms.TIMESTAMP, atoms.PNG],
                    )?
                    .ignore_error();
            } else if target == atoms.TIMESTAMP {
                self.connection
                    .change_property32(
                        PropMode::REPLACE,
                        requestor,
                        property,
                        AtomEnum::INTEGER,
                        &[self.acquired],
                    )?
                    .ignore_error();
            } else if target != atoms.PNG {
                return Ok(false);
            } else if self.png.len() <= self.chunk {
                self.connection
                    .change_property8(PropMode::REPLACE, requestor, property, atoms.PNG, &self.png)?
                    .ignore_error();
            } else {
                self.connection
                    .change_window_attributes(
                        requestor,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
                    )?
                    .ignore_error();
                self.connection
                    .change_property32(
                        PropMode::REPLACE,
                        requestor,
                        property,
                        atoms.INCR,
                        &[u32::try_from(self.png.len()).unwrap_or(u32::MAX)],
                    )?
                    .ignore_error();
                self.transfers.push(Transfer {
                    requestor,
                    property,
                    sent: 0,
                    touched: Instant::now(),
                });
            }
            Ok(true)
        }

        fn advance(&mut self, requestor: Window, property: Atom) -> Result<()> {
            let Some(index) = self.transfers.iter().position(|transfer| {
                transfer.requestor == requestor && transfer.property == property
            }) else {
                return Ok(());
            };
            let sent = self.transfers[index].sent;
            let end = (sent + self.chunk).min(self.png.len());
            self.connection
                .change_property8(
                    PropMode::REPLACE,
                    requestor,
                    property,
                    self.atoms.PNG,
                    &self.png[sent..end],
                )?
                .ignore_error();
            if sent < self.png.len() {
                let transfer = &mut self.transfers[index];
                transfer.sent = end;
                transfer.touched = Instant::now();
                return Ok(());
            }
            let _done = self.transfers.swap_remove(index);
            if self
                .transfers
                .iter()
                .all(|transfer| transfer.requestor != requestor)
            {
                self.connection
                    .change_window_attributes(
                        requestor,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::NO_EVENT),
                    )?
                    .ignore_error();
            }
            Ok(())
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod platform {
    use std::borrow::Cow;

    use anyhow::{Context as _, Result};

    use super::RgbaImage;

    pub(super) fn seize(image: RgbaImage, settle: impl FnOnce(Result<()>)) {
        settle(
            arboard::Clipboard::new()
                .and_then(|mut clipboard| {
                    clipboard.set_image(arboard::ImageData {
                        width: image.width as usize,
                        height: image.height as usize,
                        bytes: Cow::Owned(image.rgba),
                    })
                })
                .context("copy the image to the clipboard"),
        );
    }
}
