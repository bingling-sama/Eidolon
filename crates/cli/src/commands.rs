use std::path::{Path, PathBuf};

use eidolon_core::{
    assets::RawSkin,
    output::{write_output, OutputIntent},
    renderer::Renderer,
};
use log::{error, info};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;

use crate::args::{resolve_render_output, RenderAnimationArgs, SceneArgs, ViewportArgs};
use crate::preview::{PreviewApp, PreviewInteraction, PreviewPlayback};
use crate::scene::{
    apply_posture_overrides, load_character, scene_state_and_camera_from_scene, PostureOverrides,
};

pub fn run_render(
    skin: String,
    output: Option<String>,
    viewport: ViewportArgs,
    scene: SceneArgs,
    animation: RenderAnimationArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = match resolve_render_output(output, &viewport, &animation) {
        Ok(resolved) => resolved,
        Err(message) => {
            error!("{message}");
            return Err(message.into());
        }
    };

    // Reject output paths that attempt directory traversal.
    if Path::new(&output)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        error!("Output path must not contain '..' components");
        return Err(Box::from(
            "output path must not contain '..' (directory traversal)",
        ));
    }

    let output_path = PathBuf::from(&output);
    let output_intent = if animation.animate {
        OutputIntent::animation_from_path(
            &output_path,
            scene.posture.animation_preset(),
            animation.fps,
            animation.speed,
            animation.loop_count,
            (viewport.width, viewport.height),
        )?
    } else {
        OutputIntent::static_from_path(&output_path, (viewport.width, viewport.height))
    };

    info!("Minecraft skin renderer");
    info!("Skin: {}", skin);
    info!(
        "Output: {} ({}x{})",
        output, viewport.width, viewport.height
    );

    info!("Creating renderer...");
    let renderer = Renderer::new()?;
    info!("Renderer ready");

    info!("Loading character resources");
    let (mut character, camera) = load_character(&renderer, &skin, &scene)?;
    info!("Character resources loaded");

    info!("Rendering...");
    let mut frame_producer = |time| {
        if animation.animate {
            let sampled = scene.posture.animation_preset().sample(time);
            character.posture = apply_posture_overrides(sampled, &scene);
        }
        renderer.render(&character, &camera, viewport.width, viewport.height)
    };
    let written = write_output(&output_path, output_intent, &mut frame_producer)?;
    info!("Done. Saved: {}", written.display());
    Ok(())
}

pub fn run_preview(
    skin: String,
    viewport: ViewportArgs,
    scene: SceneArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let (scene_state, camera) = scene_state_and_camera_from_scene(&scene);
    let posture_overrides = PostureOverrides::from(&scene);
    let playback = PreviewPlayback::from_posture(scene.posture);

    let event_loop = EventLoop::new()?;
    let mut app = PreviewApp {
        renderer: None,
        window: None,
        character: None,
        scene_state,
        camera,
        texture_path: skin,
        cape_atlas_path: scene.cape,
        elytra_atlas_path: scene.elytra,
        initial_size: PhysicalSize::new(viewport.width, viewport.height),
        interaction: PreviewInteraction::default(),
        playback,
        posture_overrides,
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}

pub fn run_convert(input: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    match RawSkin::convert_file(&input, &output) {
        Ok(()) => {
            info!("Conversion OK. Double-layer skin saved to: {:?}", output);
            Ok(())
        }
        Err(e) => {
            error!("Conversion failed: {}", e);
            Err(Box::new(std::io::Error::other(e.to_string())))
        }
    }
}

pub fn run_identify(texture: PathBuf, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(&texture).map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("failed to read '{}': {}", texture.display(), e),
        )) as Box<dyn std::error::Error>
    })?;
    let report = eidolon_core::classifier::inspect_texture(&bytes).map_err(|e| {
        Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
    })?;

    if json {
        println!(
            "{{\n  \"kind\": \"{}\",\n  \"width\": {},\n  \"height\": {},\n  \"hd_scale\": {},\n  \"is_hd\": {},\n  \"can_auto_upgrade\": {},\n  \"description\": \"{}\"\n}}",
            report.kind.as_str(),
            report.width,
            report.height,
            report.hd_scale,
            report.is_hd,
            report.can_auto_upgrade,
            report.description
        );
    } else {
        println!("File: {}", texture.display());
        println!("Kind: {}", report.kind.as_str());
        println!("Dimensions: {}x{}", report.width, report.height);
        println!("HD: {} (scale {}x)", report.is_hd, report.hd_scale);
        println!("Auto-upgradeable: {}", report.can_auto_upgrade);
        println!("Description: {}", report.description);
    }
    Ok(())
}
