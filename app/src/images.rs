use image::ImageDecoder;
use nana_ui::{GpuContext, HostTexture, HostTextureAlphaMode, HostTextureRegistry};
use std::io::Cursor;

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
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return None;
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);
    let decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > 8 * 1024 * 1024
        || decoder.total_bytes() > 64 * 1024 * 1024
    {
        return None;
    }
    let image = image::DynamicImage::from_decoder(decoder).ok()?.to_rgba8();
    let mut rgba = image.into_raw();
    for pixel in rgba.as_chunks_mut::<4>().0 {
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
    gpu: &GpuContext,
    registry: &HostTextureRegistry,
    image: &DecodedImage,
    id: u64,
) -> wgpu::Texture {
    let device = gpu.wgpu().device();
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
    gpu.wgpu().queue().write_texture(
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
    registry.register(
        image.slot.clone(),
        HostTexture::new(id, 1, &nana_ui::GpuTexture::from_wgpu(gpu, texture.clone())),
        image.width,
        image.height,
        HostTextureAlphaMode::Premultiplied,
    );
    texture
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        use image::ImageEncoder;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }
    #[test]
    fn decodes_premultiplied_pixels_and_rejects_oversized_dimensions() {
        let decoded = decode("test", &png(1, 1, &[200, 100, 50, 128])).unwrap();
        assert_eq!(decoded.rgba, vec![100, 50, 25, 128]);
        assert_eq!((decoded.width, decoded.height), (1, 1));
        let wide = png(4097, 1, &vec![0; 4097 * 4]);
        assert!(decode("test", &wide).is_none());
        assert!(decode("test", b"invalid").is_none());
    }
}
