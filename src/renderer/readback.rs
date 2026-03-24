use image::{ImageBuffer, Rgba};

/// 创建用于读回像素的可映射 buffer，并返回 `(buffer, padded_bytes_per_row)`。
pub(crate) fn create_output_buffer(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Buffer, u32) {
    let bytes_per_pixel = 4u32;
    let unpadded_bytes_per_row = bytes_per_pixel * width;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (padded_bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    (output_buffer, padded_bytes_per_row)
}

pub(crate) fn copy_render_target_to_buffer(
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

pub(crate) fn map_output_buffer_to_rgba(
    device: &wgpu::Device,
    output_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
    let bytes_per_pixel = 4u32;

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        // Avoid panicking if the receiver is dropped; just ignore the error.
        let _ = tx.send(result);
    });
    device.poll(wgpu::PollType::Wait).ok();
    let map_result = rx
        .recv()
        .map_err(|e| -> Box<dyn std::error::Error> {
            format!("Failed to receive buffer map result: {}", e).into()
        })?;
    map_result.map_err(|e| -> Box<dyn std::error::Error> {
        format!("Buffer map failed: {:?}", e).into()
    })?;

    let data = buffer_slice.get_mapped_range();
    let mut img_buf = ImageBuffer::new(width, height);

    for y in 0..height {
        let row_start = (y * padded_bytes_per_row) as usize;
        for x in 0..width {
            let offset = row_start + (x * bytes_per_pixel) as usize;
            img_buf.put_pixel(
                x,
                y,
                Rgba([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]),
            );
        }
    }

    drop(data);
    output_buffer.unmap();

    Ok(img_buf)
}
