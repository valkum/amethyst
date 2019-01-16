//! Demonstrates how to use the fly camera

use amethyst::{
    assets::{PrefabLoader, PrefabLoaderSystem, RonFormat, Loader, AssetStorage},
    controls::FlyControlBundle,
    core::transform::{TransformBundle, Transform},
    input::{InputBundle, is_key_down},
    prelude::*,
    renderer::{DrawClipmap, Clipmap, ClipmapSystem, ActiveClipmap, DrawShaded, PosNormTex, RenderBundle, Stage, DisplayConfig, Pipeline, ShapeUpload, Mesh},
    utils::{application_root_dir, scene::BasicScenePrefab, auto_fov::{AutoFov, AutoFovSystem}},
    Error,
    winit::VirtualKeyCode,
};


struct Example;

impl SimpleState for Example {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let prefab_handle = data.world.exec(|loader: PrefabLoader<'_, BasicScenePrefab<Vec<PosNormTex>>>| {
            loader.load("prefab/test.ron", RonFormat, (), ())
        });
        data.world.register::<Transform>();
        data.world
            .create_entity()
            .named("Test")
            .with(prefab_handle)
            .build();
        
        let mut clipmap_transform = Transform::default();
        clipmap_transform.pitch_local(1.5708);
        let clipmap_entity = data.world
            .create_entity()
            .named("Clipmap")
            .with(Clipmap::new(255))
            .with(clipmap_transform)
            .build();
        data.world.add_resource(ActiveClipmap{entity: Some(clipmap_entity)});
        // world.add_resource(AmbientColor(Rgba::from([0.01; 3])));
    }
    fn handle_event(
        &mut self,
        _: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        if let StateEvent::Window(event) = event {
            if is_key_down(&event, VirtualKeyCode::Escape) {
                Trans::Quit
            } else {
                Trans::None
            }
        } else {
            Trans::None
        }
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;
    println!("Application Root: {:?}", app_root);

    let resources = app_root.join("resources");
    let display_config = resources.join("display.ron");
    
    let config = DisplayConfig::load(&display_config);

    let key_bindings_path = resources.join("input.ron");


    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.0, 1.0, 0.0, 1.0], 1.0)
            .with_pass(DrawClipmap::new())
            .with_pass(DrawShaded::<PosNormTex>::new()),
    );


    let game_data = GameDataBuilder::default()
        .with(PrefabLoaderSystem::<BasicScenePrefab<Vec<PosNormTex>>>::default(), "prefab", &[])
        // .with(AutoFovSystem, "auto_fov", &["prefab"]) // This makes the system adjust the camera right after it has been loaded (in the same frame), preventing any flickering
        // .with(ShowFovSystem, "show_fov", &["auto_fov"])
        .with(ClipmapSystem, "clipmap_system", &["prefab"])
        .with_bundle(
            FlyControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_sensitivity(0.1, 0.1),
        )?
        .with_bundle(TransformBundle::new().with_dep(&["fly_movement"]))?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(RenderBundle::new(pipe, Some(config)))?;

    let mut game = Application::new(resources, Example, game_data)?;

    game.run();

    Ok(())
}