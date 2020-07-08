use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::{LocalToWorld, Translation},
    prelude::*,
    renderer::{
        camera::Projection, Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat,
        Texture,
    },
};

const ARENA_HEIGHT: f32 = 100.0;
const ARENA_WIDTH: f32 = 100.0;

const PADDLE_HEIGHT: f32 = 16.0;
const PADDLE_WIDTH: f32 = 4.0;

pub struct Pong;

impl SimpleState for Pong {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        let StateData {
            world, resources, ..
        } = data;

        // Load the spritesheet necessary to render the graphics.
        // `spritesheet` is the layout of the sprites on the image;
        // `texture` is the pixel data.
        let sprite_sheet_handle = load_sprite_sheet(resources);

        /* world.register::<Paddle>(); */

        initialise_paddles(world, sprite_sheet_handle);
        initialise_camera(world);
    }
}

#[derive(PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

struct Paddle {
    pub side: Side,
    pub width: f32,
    pub height: f32,
}

impl Paddle {
    fn new(side: Side) -> Paddle {
        Paddle {
            side,
            width: PADDLE_WIDTH,
            height: PADDLE_HEIGHT,
        }
    }
}

fn load_sprite_sheet(resources: &mut Resources) -> Handle<SpriteSheet> {
    let texture_handle = {
        let loader = resources
            .get::<Loader>()
            .expect("Could not get Loader resource");

        let texture_storage = resources.get::<AssetStorage<Texture>>().unwrap();
        loader.load(
            "texture/pong_spritesheet.png",
            ImageFormat::default(),
            (),
            &texture_storage,
        )
    };
    let loader = resources
        .get::<Loader>()
        .expect("Could not get Loader resource");
    let sprite_sheet_store = resources.get::<AssetStorage<SpriteSheet>>().unwrap();
    loader.load(
        "texture/pong_spritesheet.ron",
        SpriteSheetFormat(texture_handle),
        (),
        &sprite_sheet_store,
    )
}

/// Initialise the camera.
fn initialise_camera(world: &mut World) {
    let translation = Translation::new(ARENA_WIDTH * 0.5, ARENA_HEIGHT * 0.5, 1.0);

    world.insert((), vec![(LocalToWorld::identity(), Camera::standard_2d(ARENA_WIDTH, ARENA_HEIGHT), translation)]);
}

/// Initialises one paddle on the left, and one paddle on the right.
fn initialise_paddles(world: &mut World, sprite_sheet_handle: Handle<SpriteSheet>) {
    // Correctly position the paddles.
    let y = ARENA_HEIGHT / 2.0;
    let left_translation = Translation::new(PADDLE_WIDTH * 0.5, y, 0.0);
    let right_translation = Translation::new(ARENA_WIDTH - PADDLE_WIDTH * 0.5, y, 0.0);

    // Assign the sprites for the paddles
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet_handle,
        sprite_number: 0, // paddle is the first sprite in the sprite_sheet
    };

    // Create a left plank entity.
    world.insert(
        (),
        vec![(
            LocalToWorld::identity(),
            sprite_render.clone(),
            Paddle::new(Side::Left),
            left_translation,
        )],
    );
    // Create right plank entity.
    world.insert(
        (),
        vec![(
            LocalToWorld::identity(),
            sprite_render,
            Paddle::new(Side::Right),
            right_translation,
        )],
    );
}
