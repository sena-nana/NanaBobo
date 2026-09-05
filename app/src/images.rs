use nana_ui::{HostTexture, HostTextureAlphaMode, HostTextureRegistry, HostedGpuResources};

pub const ACCOUNT_AVATAR: &str = "account-avatar";
pub const ROOM_AVATAR: &str = "room-avatar";
pub const ROOM_COVER: &str = "room-cover";

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub slot: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode(slot: impl Into<String>, bytes: &[u8]) -> Option<DecodedImage> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let width = image.width().max(1);
    let height = image.height().max(1);
    let mut rgba = image.into_raw();
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        pixel[0] = ((u16::from(pixel[0]) * alpha) / 255) as u8;
        pixel[1] = ((u16::from(pixel[1]) * alpha) / 255) as u8;
        pixel[2] = ((u16::from(pixel[2]) * alpha) / 255) as u8;
    }
    Some(DecodedImage {
        slot: slot.into(),
        width,
        height,
        rgba,
    })
}

pub fn upload(
    gpu: &HostedGpuResources,
    registry: &HostTextureRegistry,
    image: &DecodedImage,
    id: u64,
) -> wgpu::Texture {
    let device = gpu.device();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("nanabobo host image"),
        size: wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width * 4),
            rows_per_image: Some(image.height),
        },
        wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    registry.register(
        image.slot.clone(),
        HostTexture::from_wgpu(id, 1, view),
        image.width,
        image.height,
        HostTextureAlphaMode::Premultiplied,
    );
    texture
}
