use crate::error::EidolonError;
use image::{ImageBuffer, Rgba};

const BYTES_PER_PIXEL: u32 = 4;

#[derive(Debug)]
struct OutputLayout {
    padded_bytes_per_row: u32,
    buffer_size: u64,
}

pub(crate) fn validate_dimensions(width: u32, height: u32) -> Result<(), EidolonError> {
    output_layout(width, height).map(|_| ())
}

fn output_layout(width: u32, height: u32) -> Result<OutputLayout, EidolonError> {
    if width == 0 || height == 0 {
        return Err(EidolonError::gpu(
            "readback dimensions must be greater than zero",
        ));
    }

    let unpadded_bytes_per_row = BYTES_PER_PIXEL
        .checked_mul(width)
        .ok_or_else(|| EidolonError::gpu("buffer size overflow: width too large"))?;
    let padded_bytes_per_row = unpadded_bytes_per_row
        .checked_next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        .ok_or_else(|| EidolonError::gpu("buffer size overflow: padded row size too large"))?;
    let buffer_size = u64::from(padded_bytes_per_row)
        .checked_mul(u64::from(height))
        .ok_or_else(|| EidolonError::gpu("buffer size overflow: dimensions too large"))?;

    Ok(OutputLayout {
        padded_bytes_per_row,
        buffer_size,
    })
}

fn create_output_buffer(device: &wgpu::Device, layout: &OutputLayout) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: layout.buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

fn copy_render_target_to_buffer(
    encoder: &mut wgpu::CommandEncoder,
    render_texture: &wgpu::Texture,
    output_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: render_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn submit_texture_copy(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<(wgpu::Buffer, u32), EidolonError> {
    let layout = output_layout(width, height)?;
    let output_buffer = create_output_buffer(device, &layout);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Readback Encoder"),
    });
    copy_render_target_to_buffer(
        &mut encoder,
        render_texture,
        &output_buffer,
        width,
        height,
        layout.padded_bytes_per_row,
    );
    queue.submit(Some(encoder.finish()));
    Ok((output_buffer, layout.padded_bytes_per_row))
}

/// Read a completed render texture into an RGBA image.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_texture_to_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
    let (output_buffer, padded_bytes_per_row) =
        submit_texture_copy(device, queue, render_texture, width, height)?;
    map_output_buffer_to_rgba(device, &output_buffer, width, height, padded_bytes_per_row)
}

/// Read a completed render texture into an RGBA image without blocking the browser thread.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn read_texture_to_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
    let (output_buffer, padded_bytes_per_row) =
        submit_texture_copy(device, queue, render_texture, width, height)?;
    map_output_buffer_to_rgba(device, &output_buffer, width, height, padded_bytes_per_row).await
}

#[cfg(not(target_arch = "wasm32"))]
fn map_output_buffer_to_rgba(
    device: &wgpu::Device,
    output_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device
        .poll(wgpu::PollType::Wait)
        .map_err(|e| EidolonError::gpu(format!("device poll failed: {e}")))?;
    let map_result = rx
        .recv()
        .map_err(|e| EidolonError::gpu(format!("failed to receive buffer map result: {e}")))?;
    map_result.map_err(|e| EidolonError::gpu(format!("buffer map failed: {e:?}")))?;

    let data = buffer_slice.get_mapped_range();
    let image = image_from_padded_rgba(&data, width, height, padded_bytes_per_row);
    drop(data);
    output_buffer.unmap();
    image
}

#[cfg(target_arch = "wasm32")]
async fn map_output_buffer_to_rgba(
    device: &wgpu::Device,
    output_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = futures_channel::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    // On WASM, PollType::Wait yields to the browser event loop via requestAnimationFrame.
    device
        .poll(wgpu::PollType::Wait)
        .map_err(|e| EidolonError::gpu(format!("device poll failed: {e}")))?;
    let map_result = rx
        .await
        .map_err(|_| EidolonError::gpu("oneshot channel closed"))?;
    map_result.map_err(|e| EidolonError::gpu(format!("buffer map failed: {e:?}")))?;

    let data = buffer_slice.get_mapped_range();
    let image = image_from_padded_rgba(&data, width, height, padded_bytes_per_row);
    drop(data);
    output_buffer.unmap();
    image
}

fn image_from_padded_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, EidolonError> {
    validate_dimensions(width, height)?;
    let unpadded_bytes_per_row = BYTES_PER_PIXEL
        .checked_mul(width)
        .ok_or_else(|| EidolonError::gpu("buffer size overflow: width too large"))?;
    if padded_bytes_per_row < unpadded_bytes_per_row {
        return Err(EidolonError::gpu(
            "mapped buffer row is smaller than the image row",
        ));
    }
    let required_len = u64::from(padded_bytes_per_row)
        .checked_mul(u64::from(height))
        .ok_or_else(|| EidolonError::gpu("mapped buffer size overflow"))?;
    if required_len > data.len() as u64 {
        return Err(EidolonError::gpu("mapped buffer is shorter than expected"));
    }

    let mut image = ImageBuffer::new(width, height);
    for y in 0..height {
        let row_start = (u64::from(y) * u64::from(padded_bytes_per_row)) as usize;
        for x in 0..width {
            let offset = row_start + (x * BYTES_PER_PIXEL) as usize;
            image.put_pixel(
                x,
                y,
                Rgba([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]),
            );
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_layout_uses_copy_alignment() {
        let layout = output_layout(256, 256).unwrap();
        assert_eq!(
            layout.padded_bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
            0
        );
        assert_eq!(layout.padded_bytes_per_row, 1_024);
        assert_eq!(layout.buffer_size, 1_024 * 256);
    }

    #[test]
    fn output_layout_rejects_zero_dimensions() {
        assert!(output_layout(0, 2).is_err());
        assert!(output_layout(2, 0).is_err());
    }

    #[test]
    fn output_layout_rejects_overflow() {
        assert!(output_layout(u32::MAX, 100)
            .unwrap_err()
            .to_string()
            .contains("width too large"));
        assert!(output_layout(0x3FFFFFFF, 100)
            .unwrap_err()
            .to_string()
            .contains("padded row size too large"));
    }

    #[test]
    fn image_from_padded_rgba_strips_each_row_padding() {
        let padded_bytes_per_row = 256;
        let mut data = vec![0; padded_bytes_per_row * 2];
        data[..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        data[padded_bytes_per_row..padded_bytes_per_row + 8]
            .copy_from_slice(&[9, 10, 11, 12, 13, 14, 15, 16]);

        let image = image_from_padded_rgba(&data, 2, 2, padded_bytes_per_row as u32).unwrap();
        assert_eq!(
            image.as_raw(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn image_from_padded_rgba_rejects_short_buffer() {
        let result = image_from_padded_rgba(&[0; 8], 2, 2, 256);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("shorter than expected"));
    }
}
