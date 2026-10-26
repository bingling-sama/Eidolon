use ab_glyph::{point, Font, FontRef, GlyphId, ScaleFont};
use cgmath::{Matrix4, Point3, SquareMatrix, Transform, Vector3, Vector4};
use std::sync::Arc;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::character::Character;
use crate::model::TexturedVertex;
use crate::texture::Texture;

use super::uniforms::{compute_base_model_matrix, Uniforms};

const PRIMARY_FONT_BYTES: &[u8] = include_bytes!("../../../../resources/minecraft.ttf");
const FALLBACK_FONT_BYTES: &[u8] = include_bytes!("../../../../resources/unifont.otf");
const FONT_HEIGHT: f32 = 16.0;
const HORIZONTAL_PADDING: u32 = 4;
const VERTICAL_PADDING: u32 = 2;
const SHADOW_OFFSET: u32 = 1;
const MAX_CHARACTERS: usize = 64;
const WORLD_UNITS_PER_PIXEL: f32 = 0.01;
const NAMETAG_VERTICAL_GAP: f32 = 0.5;
const COLLISION_BOX_MIN: Point3<f32> = Point3::new(-0.5, 0.0, -0.25);
const COLLISION_BOX_MAX: Point3<f32> = Point3::new(0.5, 1.8, 0.25);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NametagPass {
    SeeThrough,
    Normal,
}

impl NametagPass {
    pub fn depth_compare(self) -> wgpu::CompareFunction {
        match self {
            Self::SeeThrough => wgpu::CompareFunction::Always,
            Self::Normal => wgpu::CompareFunction::LessEqual,
        }
    }

    pub fn fragment_entry(self) -> &'static str {
        match self {
            Self::SeeThrough => "fs_see_through",
            Self::Normal => "fs_main",
        }
    }
}

pub(crate) fn nametag_passes(see_through: bool) -> &'static [NametagPass] {
    const VANILLA_PASSES: &[NametagPass] = &[NametagPass::SeeThrough, NametagPass::Normal];
    const OCCLUDED_PASSES: &[NametagPass] = &[NametagPass::Normal];

    if see_through {
        VANILLA_PASSES
    } else {
        OCCLUDED_PASSES
    }
}

pub(crate) struct CachedNametag {
    #[allow(dead_code)]
    pub text: String,
    #[allow(dead_code)]
    pub context_id: u64,
    pub texture: Texture,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
}

impl CachedNametag {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        text: &str,
        context_id: u64,
    ) -> Option<Arc<Self>> {
        let image = rasterize_nametag(text)?;
        let texture = Texture::upload_raw(
            device,
            queue,
            texture_bind_group_layout,
            sampler,
            &image.rgba,
            image.width,
            image.height,
        )
        .expect("generated nametag texture dimensions are valid");
        let vertices = nametag_vertices(image.width, image.height);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Nametag Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Some(Arc::new(Self {
            text: text.to_owned(),
            context_id,
            texture,
            vertex_buffer,
            vertex_count: vertices.len() as u32,
        }))
    }
}

pub(crate) struct NametagImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub(crate) fn rasterize_nametag(text: &str) -> Option<NametagImage> {
    if text.trim().is_empty() {
        return None;
    }

    let text: String = text
        .chars()
        .take(MAX_CHARACTERS)
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let primary_font =
        FontRef::try_from_slice(PRIMARY_FONT_BYTES).expect("embedded minecraft font is valid");
    let fallback_font =
        FontRef::try_from_slice(FALLBACK_FONT_BYTES).expect("embedded unifont fallback is valid");
    let primary_scaled = primary_font.as_scaled(FONT_HEIGHT);
    let fallback_scaled = fallback_font.as_scaled(FONT_HEIGHT);

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum FontSource {
        Primary,
        Fallback,
    }

    let baseline = VERTICAL_PADDING as f32 + primary_scaled.ascent().ceil();
    let mut caret = HORIZONTAL_PADDING as f32;
    let mut previous: Option<(FontSource, GlyphId)> = None;
    let mut glyphs = Vec::new();

    for character in text.chars() {
        let primary_id = primary_scaled.glyph_id(character);
        let (source, scaled, id) = if primary_id.0 != 0 {
            (FontSource::Primary, &primary_scaled, primary_id)
        } else {
            let fallback_id = fallback_scaled.glyph_id(character);
            if fallback_id.0 != 0 {
                (FontSource::Fallback, &fallback_scaled, fallback_id)
            } else {
                (FontSource::Primary, &primary_scaled, primary_id)
            }
        };

        if let Some((prev_source, prev_id)) = previous {
            if prev_source == source {
                caret += scaled.kern(prev_id, id);
            }
        }
        let glyph = id.with_scale_and_position(FONT_HEIGHT, point(caret, baseline));
        if let Some(outlined) = scaled.outline_glyph(glyph) {
            glyphs.push(outlined);
        }
        caret += scaled.h_advance(id);
        previous = Some((source, id));
    }

    let width = (caret.ceil() as u32 + HORIZONTAL_PADDING + SHADOW_OFFSET).max(1);
    let height = (FONT_HEIGHT.ceil() as u32 + VERTICAL_PADDING * 2 + SHADOW_OFFSET).max(1);
    let mut rgba = vec![0; (width * height * 4) as usize];
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[0, 0, 0, 64]);
    }

    draw_glyphs(
        &mut rgba,
        width,
        height,
        &glyphs,
        SHADOW_OFFSET,
        SHADOW_OFFSET,
        [63, 63, 63, 255],
    );
    draw_glyphs(
        &mut rgba,
        width,
        height,
        &glyphs,
        0,
        0,
        [255, 255, 255, 255],
    );

    Some(NametagImage {
        rgba,
        width,
        height,
    })
}

fn draw_glyphs(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    glyphs: &[ab_glyph::OutlinedGlyph],
    offset_x: u32,
    offset_y: u32,
    color: [u8; 4],
) {
    for glyph in glyphs {
        let bounds = glyph.px_bounds();
        glyph.draw(|x, y, coverage| {
            if coverage < 0.5 {
                return;
            }
            let x = bounds.min.x as i32 + x as i32 + offset_x as i32;
            let y = bounds.min.y as i32 + y as i32 + offset_y as i32;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                return;
            }
            let index = ((y as u32 * width + x as u32) * 4) as usize;
            rgba[index..index + 4].copy_from_slice(&color);
        });
    }
}

fn nametag_vertices(width: u32, height: u32) -> [TexturedVertex; 6] {
    let half_width = width as f32 * WORLD_UNITS_PER_PIXEL * 0.5;
    let half_height = height as f32 * WORLD_UNITS_PER_PIXEL * 0.5;
    let vertex = |x, y, u, v| TexturedVertex {
        position: [x, y, 0.0],
        normal: [0.0, 0.0, 1.0],
        texture: [u, v],
    };

    [
        vertex(-half_width, half_height, 0.0, 0.0),
        vertex(-half_width, -half_height, 0.0, 1.0),
        vertex(half_width, -half_height, 1.0, 1.0),
        vertex(-half_width, half_height, 0.0, 0.0),
        vertex(half_width, -half_height, 1.0, 1.0),
        vertex(half_width, half_height, 1.0, 0.0),
    ]
}

pub(crate) fn compute_nametag_uniform(
    character: &Character,
    camera: &Camera,
    width: u32,
    height: u32,
) -> Uniforms {
    let base = compute_base_model_matrix(character);
    let bounds = transformed_collision_box_bounds(base);
    let anchor = Point3::new(
        (bounds.min.x + bounds.max.x) * 0.5,
        bounds.max.y + NAMETAG_VERTICAL_GAP,
        (bounds.min.z + bounds.max.z) * 0.5,
    );
    let view: Matrix4<f32> = camera.get_view_matrix().into();
    let inverse_view = view.invert().expect("camera view matrix is invertible");
    let billboard_rotation = Matrix4::from_cols(
        Vector4::new(inverse_view.x.x, inverse_view.x.y, inverse_view.x.z, 0.0),
        Vector4::new(inverse_view.y.x, inverse_view.y.y, inverse_view.y.z, 0.0),
        Vector4::new(inverse_view.z.x, inverse_view.z.y, inverse_view.z.z, 0.0),
        Vector4::unit_w(),
    );
    let model =
        Matrix4::from_translation(Vector3::new(anchor.x, anchor.y, anchor.z)) * billboard_rotation;

    Uniforms {
        perspective: camera.get_projection_matrix(width, height),
        view: view.into(),
        model: model.into(),
        offset: 0.0,
        _padding: [0.0; 3],
    }
}

#[derive(Clone, Copy, Debug)]
struct SceneBounds {
    min: Point3<f32>,
    max: Point3<f32>,
}

fn transformed_collision_box_bounds(base: Matrix4<f32>) -> SceneBounds {
    let mut bounds = SceneBounds {
        min: Point3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        max: Point3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    for x in [COLLISION_BOX_MIN.x, COLLISION_BOX_MAX.x] {
        for y in [COLLISION_BOX_MIN.y, COLLISION_BOX_MAX.y] {
            for z in [COLLISION_BOX_MIN.z, COLLISION_BOX_MAX.z] {
                let point = base.transform_point(Point3::new(x, y, z));
                bounds.min.x = bounds.min.x.min(point.x);
                bounds.min.y = bounds.min.y.min(point.y);
                bounds.min.z = bounds.min.z.min(point.z);
                bounds.max.x = bounds.max.x.max(point.x);
                bounds.max.y = bounds.max.y.max(point.y);
                bounds.max.z = bounds.max.z.max(point.z);
            }
        }
    }

    bounds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterized_nametag_has_vanilla_background_text_and_shadow() {
        let image = rasterize_nametag("Steve").expect("non-empty nametag");

        assert!(image.width > image.height);
        assert!(image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel == &[0, 0, 0, 64]));
        assert!(image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel == &[255, 255, 255, 255]));
        assert!(image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel == &[63, 63, 63, 255]));
    }

    #[test]
    fn rasterized_nametag_falls_back_to_unifont_for_cjk() {
        let image = rasterize_nametag("史蒂夫").expect("cjk nametag");
        assert!(image.width > image.height);
        assert!(image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel == &[255, 255, 255, 255]));
        assert!(image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel == &[63, 63, 63, 255]));
    }

    #[test]
    fn empty_nametags_are_not_rasterized() {
        assert!(rasterize_nametag("").is_none());
        assert!(rasterize_nametag("   ").is_none());
    }

    #[test]
    fn nametag_billboard_anchor_follows_character_position() {
        let mut character = Character::empty_for_test();
        character.position = Vector3::new(1.0, 2.0, 3.0);
        let uniform = compute_nametag_uniform(&character, &Camera::default(), 800, 600);
        let model: Matrix4<f32> = uniform.model.into();
        let anchor = model.transform_point(Point3::new(0.0, 0.0, 0.0));

        assert!((anchor.x - 1.0).abs() < 1e-6);
        assert!((anchor.y - 4.3).abs() < 1e-6);
        assert!((anchor.z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn nametag_anchor_is_above_the_scene_space_collision_box() {
        let camera = Camera::default();

        for posture in [
            crate::character::DefaultPostures::STAND,
            crate::character::DefaultPostures::FLYING,
        ] {
            let mut character = Character::empty_for_test();
            character.posture = posture;
            character.position = Vector3::new(1.0, 2.0, 3.0);
            character.rotation = Vector3::new(10.0, 25.0, 5.0);

            let base = compute_base_model_matrix(&character);
            let corners = [-0.5, 0.5].into_iter().flat_map(|x| {
                [0.0, 1.8].into_iter().flat_map(move |y| {
                    [-0.25, 0.25]
                        .into_iter()
                        .map(move |z| base.transform_point(Point3::new(x, y, z)))
                })
            });
            let (min, max) = corners.fold(
                (
                    Point3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
                    Point3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                ),
                |(min, max), point| {
                    (
                        Point3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z)),
                        Point3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z)),
                    )
                },
            );
            let expected_anchor = Point3::new(
                (min.x + max.x) * 0.5,
                max.y + NAMETAG_VERTICAL_GAP,
                (min.z + max.z) * 0.5,
            );
            let uniform = compute_nametag_uniform(&character, &camera, 800, 600);
            let model: Matrix4<f32> = uniform.model.into();
            let actual_anchor = model.transform_point(Point3::new(0.0, 0.0, 0.0));

            assert!((actual_anchor.x - expected_anchor.x).abs() < 1e-6);
            assert!((actual_anchor.y - expected_anchor.y).abs() < 1e-6);
            assert!((actual_anchor.z - expected_anchor.z).abs() < 1e-6);
        }
    }

    #[test]
    fn see_through_configuration_selects_vanilla_two_pass_rendering() {
        assert_eq!(
            nametag_passes(true),
            &[NametagPass::SeeThrough, NametagPass::Normal]
        );
        assert_eq!(nametag_passes(false), &[NametagPass::Normal]);
        assert_eq!(
            NametagPass::SeeThrough.depth_compare(),
            wgpu::CompareFunction::Always
        );
        assert_eq!(NametagPass::SeeThrough.fragment_entry(), "fs_see_through");
    }
}
