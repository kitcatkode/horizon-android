//! A second display's surface (the AYN Thor's lower screen): a HUD batch drawn straight into its
//! own swapchain on the game's device, after the main frame. Nothing of the world is drawn
//! there, so it needs only a HUD renderer for its format and a depth target for item models.
use super::{Renderer, surface_format, upload::Uploads};
use crate::{
    Result,
    hud::{HudBatch, HudRenderer},
};

pub struct Panel {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    config: wgpu::SurfaceConfiguration,
    /// Built from the game's HUD renderer and rebuilt when its atlas changes.
    hud: Option<HudRenderer>,
    depth: Option<wgpu::TextureView>,
    uploads: Uploads,
}

impl Renderer {
    /// The instance a panel's surface is made with (the Android host makes it from its window).
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// A panel presenting on `surface`, made with `instance()`, on this renderer's device.
    pub fn panel(&self, surface: wgpu::Surface<'static>, size: [u32; 2]) -> Result<Panel> {
        Panel::new(surface, &self.adapter, &self.device, size)
    }

    /// The panel was made on this renderer's device (a new renderer needs a new panel).
    pub fn owns_panel(&self, panel: &Panel) -> bool {
        panel.device == self.device
    }

    /// Draw `batch` on the panel over `background`. False when no frame was shown (the surface
    /// was outdated or busy; the next call tries again).
    pub fn draw_panel(
        &self,
        panel: &mut Panel,
        batch: &HudBatch,
        background: wgpu::Color,
    ) -> Result<bool> {
        if panel.device != self.device {
            return Err("the panel belongs to another device".into());
        }
        if panel.hud.as_ref().map(|h| &h.atlas_texture) != Some(&self.hud.atlas_texture) {
            let format = panel.config.format.remove_srgb_suffix();
            panel.hud = Some(self.hud.sibling(&self.device, format)?);
        }
        let frame = match panel.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                drop(frame);
                panel.configure();
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Validation => {
                panel.configure();
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(false);
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err("the panel's surface was lost".into()),
        };
        let size = [panel.config.width, panel.config.height];
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(panel.config.format.remove_srgb_suffix()),
            ..Default::default()
        });
        let depth = panel.depth(size).clone();
        let hud = panel.hud.as_mut().expect("built above");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Panel"),
            });
        hud.prepare(&self.device, &mut panel.uploads, &mut encoder, batch, size);
        panel.uploads.finish(&encoder);
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Panel background"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(background),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        hud.draw(&mut encoder, &view, &depth, batch, size);
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(true)
    }
}

impl Panel {
    fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        size: [u32; 2],
    ) -> Result<Self> {
        let max = device.limits().max_texture_dimension_2d;
        let [width, height] = size.map(|v| v.clamp(1, max));
        let mut config = surface
            .get_default_config(adapter, width, height)
            .ok_or("the panel's surface has no supported configuration")?;
        let caps = surface.get_capabilities(adapter);
        if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        }
        config.format = surface_format(&caps.formats, config.format);
        config.view_formats = vec![config.format.remove_srgb_suffix()];
        // Never wait on the panel's refresh: the game's frame pacing belongs to the main screen.
        config.present_mode = [wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
            .into_iter()
            .find(|mode| caps.present_modes.contains(mode))
            .unwrap_or(wgpu::PresentMode::Fifo);
        config.desired_maximum_frame_latency = 1;
        let panel = Self {
            surface,
            device: device.clone(),
            config,
            hud: None,
            depth: None,
            uploads: Uploads::new(device),
        };
        panel.configure();
        Ok(panel)
    }

    fn configure(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    /// The surface's size, in pixels.
    pub fn size(&self) -> [u32; 2] {
        [self.config.width, self.config.height]
    }

    /// Follow the display's new size (rotation, a resized surface).
    pub fn resize(&mut self, size: [u32; 2]) {
        let max = self.device.limits().max_texture_dimension_2d;
        let [width, height] = size.map(|v| v.clamp(1, max));
        if [width, height] != self.size() {
            self.config.width = width;
            self.config.height = height;
            self.depth = None;
            self.configure();
        }
    }

    /// The depth target `Blend::Depth` runs (item models) test against.
    fn depth(&mut self, size: [u32; 2]) -> &wgpu::TextureView {
        self.depth.get_or_insert_with(|| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Panel depth"),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        })
    }
}
