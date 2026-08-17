//! Native X11 bitmap cursors, independent of winit's broken Xcursor bridge.

use anyhow::{Context as _, Result, bail};
use egui::CustomCursorImage;
use std::sync::Arc;
use winit::{
    raw_window_handle::{HasWindowHandle as _, RawWindowHandle},
    window::Window,
};
use x11rb::{
    connection::Connection as _,
    protocol::{
        render::{ConnectionExt as _, CreatePictureAux, PictType, Pictformat},
        xproto::{
            ChangeWindowAttributesAux, ConnectionExt as _, CreateGCAux, ImageFormat, ImageOrder,
        },
    },
    rust_connection::RustConnection,
};

pub(crate) struct X11CursorFoundry {
    forge: Option<Forge>,
}

struct Forge {
    connection: RustConnection,
    root: u32,
    window: u32,
    argb32: Pictformat,
    installed: Option<(usize, u32)>,
}

impl X11CursorFoundry {
    pub(crate) fn bind(window: &Window) -> Result<Self> {
        let raw = window
            .window_handle()
            .context("read native window handle")?
            .as_raw();
        let window = match raw {
            RawWindowHandle::Xlib(handle) => {
                u32::try_from(handle.window).context("Xlib window identifier exceeds X11 range")?
            }
            RawWindowHandle::Xcb(handle) => handle.window.get(),
            _ => return Ok(Self { forge: None }),
        };
        let (connection, screen) = x11rb::connect(None).context("connect X11 cursor foundry")?;
        let root = connection.setup().roots[screen].root;
        let formats = connection
            .render_query_pict_formats()
            .context("query XRender picture formats")?
            .reply()
            .context("read XRender picture formats")?;
        let argb32 = formats
            .formats
            .iter()
            .find(|format| {
                format.type_ == PictType::DIRECT
                    && format.depth == 32
                    && format.direct.red_shift == 16
                    && format.direct.red_mask == 0xff
                    && format.direct.green_shift == 8
                    && format.direct.green_mask == 0xff
                    && format.direct.blue_shift == 0
                    && format.direct.blue_mask == 0xff
                    && format.direct.alpha_shift == 24
                    && format.direct.alpha_mask == 0xff
            })
            .map(|format| format.id)
            .context("XRender server has no ARGB32 picture format")?;
        Ok(Self {
            forge: Some(Forge {
                connection,
                root,
                window,
                argb32,
                installed: None,
            }),
        })
    }

    pub(crate) fn apply(&mut self, image: Option<&CustomCursorImage>) -> Result<()> {
        let Some(forge) = &mut self.forge else {
            return Ok(());
        };
        match image {
            Some(image) => forge.strike(image),
            None => forge.release(),
        }
    }
}

impl Forge {
    fn strike(&mut self, image: &CustomCursorImage) -> Result<()> {
        let key = Arc::as_ptr(&image.rgba).cast::<u8>() as usize;
        if self
            .installed
            .is_some_and(|(installed, _)| installed == key)
        {
            return Ok(());
        }
        let [width, height] = image.size;
        let expected = usize::from(width) * usize::from(height) * 4;
        if width == 0
            || height == 0
            || image.hotspot[0] >= width
            || image.hotspot[1] >= height
            || image.rgba.len() != expected
        {
            bail!("invalid {width}x{height} RGBA cursor payload");
        }
        let pixmap = self
            .connection
            .generate_id()
            .context("allocate cursor pixmap")?;
        let gc = self
            .connection
            .generate_id()
            .context("allocate cursor graphics context")?;
        let picture = self
            .connection
            .generate_id()
            .context("allocate cursor picture")?;
        let cursor = self.connection.generate_id().context("allocate cursor")?;
        self.connection
            .create_pixmap(32, pixmap, self.root, width, height)
            .context("create cursor pixmap")?
            .check()
            .context("establish cursor pixmap")?;
        self.connection
            .create_gc(gc, pixmap, &CreateGCAux::new())
            .context("create cursor graphics context")?
            .check()
            .context("establish cursor graphics context")?;
        let pixels = argb_bytes(
            &image.rgba,
            self.connection.setup().image_byte_order == ImageOrder::LSB_FIRST,
        );
        self.connection
            .put_image(
                ImageFormat::Z_PIXMAP,
                pixmap,
                gc,
                width,
                height,
                0,
                0,
                0,
                32,
                &pixels,
            )
            .context("upload cursor pixels")?
            .check()
            .context("establish cursor pixels")?;
        self.connection
            .free_gc(gc)
            .context("free cursor graphics context")?;
        self.connection
            .render_create_picture(picture, pixmap, self.argb32, &CreatePictureAux::new())
            .context("create cursor picture")?
            .check()
            .context("establish cursor picture")?;
        self.connection
            .render_create_cursor(cursor, picture, image.hotspot[0], image.hotspot[1])
            .context("create XRender cursor")?
            .check()
            .context("establish XRender cursor")?;
        self.connection
            .render_free_picture(picture)
            .context("free cursor picture")?;
        self.connection
            .free_pixmap(pixmap)
            .context("free cursor pixmap")?;
        self.connection
            .change_window_attributes(
                self.window,
                &ChangeWindowAttributesAux::new().cursor(cursor),
            )
            .context("install cursor on X11 window")?
            .check()
            .context("establish cursor on X11 window")?;
        self.connection.flush().context("flush X11 cursor")?;
        if let Some((_, old)) = self.installed.replace((key, cursor)) {
            self.connection
                .free_cursor(old)
                .context("free replaced cursor")?;
        }
        Ok(())
    }

    fn release(&mut self) -> Result<()> {
        if let Some((_, cursor)) = self.installed.take() {
            self.connection
                .free_cursor(cursor)
                .context("free released cursor")?;
            self.connection.flush().context("flush released cursor")?;
        }
        Ok(())
    }
}

fn argb_bytes(rgba: &[u8], little_endian: bool) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|pixel| {
            let argb = u32::from(pixel[3]) << 24
                | u32::from(pixel[0]) << 16
                | u32::from(pixel[1]) << 8
                | u32::from(pixel[2]);
            if little_endian {
                argb.to_le_bytes()
            } else {
                argb.to_be_bytes()
            }
        })
        .collect()
}
