use amethyst_assets::{Handle, PrefabData, PrefabError, Loader, AssetStorage};
use amethyst_core::{
    transform::GlobalTransform,
    specs::{
        Component, Entity, HashMapStorage, Join, ReadExpect, Read, ReadStorage, System, Write, WriteStorage,
    }
};
use crate::{
    error::Result,
    light::Light,
    pipe::{
        pass::{Pass, PassData},
        DepthMode, Effect, NewEffect,
    },
    Separate,
    pass::{
        shaded_util::{set_light_args, setup_light_buffers},
        util::{get_camera, setup_textures, setup_vertex_args, set_attribute_buffers, set_view_args, ViewArgs},
    },
    resources::AmbientColor,
    ActiveCamera, Camera, Encoder, Factory, Mesh, PosTex, Rgba, Shape, ComboMeshCreator, build_mesh_with_combo, ShapeUpload,
    VertexFormat, 
    vertex::{Attributes, Position, TexCoord},
    MeshData,
    TextureMetadata, Texture, SamplerInfo, WrapMode, FilterMethod, SurfaceType,
    PngFormat,
};

use gfx::pso::buffer::ElemStride;
use glsl_layout::{mat4, Uniform, vec4};
use gfx_core::format::ChannelType;
use std::mem;
use genmesh as gm;
use genmesh::generators::{SharedVertex, IndexedPolygon};
use std::{
    error::Error as StdError,
    result::Result as StdResult,
};
// use super::ClipmapParams;

type ClipmapMeshHandle = Handle<Mesh>;

#[derive(Clone, PrefabData)]
#[prefab(Component)]
// #[serde(default)]
pub struct Clipmap{
    block_mesh: Option<ClipmapMeshHandle>,
    fixup_mesh: Option<ClipmapMeshHandle>,
    trim_mesh: Option<[ClipmapMeshHandle; 4]>,
    elevation: Option<Handle<Texture>>,
    z_color: Option<Handle<Texture>>,
    size: u32,
    alpha_offset: [f32; 2],
    one_over_width: [f32; 2],

}
impl Clipmap {
    /// Creates a new instance with the default values for all fields
    pub fn new(size: u32) -> Self {
        let transition_width = size as f32/10.;

        Clipmap {
            block_mesh: None,
            fixup_mesh: None,
            trim_mesh: None,
            elevation: None,
            z_color: None,
            size: size,
            // Per forumla this hould be: (n-1)/2-w-1 with w = transition width (n/10)
            alpha_offset: [(size - 1) as f32/2. - transition_width - 1.; 2],
            one_over_width: [1. / (size as f32/10.); 2],
        }
    }



    pub fn get_block_mesh(&self) -> Option<&ClipmapMeshHandle> { self.block_mesh.as_ref() }

    pub fn get_uniforms(&self) -> (u32, [f32; 2], [f32; 2]) {(self.size, self.alpha_offset, self.one_over_width)}
    pub fn get_elevation(&self) -> Option<&Handle<Texture>> {self.elevation.as_ref()}
    pub fn get_z_color(&self) -> Option<&Handle<Texture>> {self.z_color.as_ref()}
}
impl Component for Clipmap {
    type Storage = HashMapStorage<Self>;
}
impl Default for Clipmap {
    fn default() -> Self {
        Clipmap::new(15)
    }
}

/// Active clipmap resource, used by the renderer to choose which camera to get the view matrix from.
/// If no active camera is found, the first camera will be used as a fallback.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ActiveClipmap {
    /// Camera entity
    pub entity: Option<Entity>,
}


/// Active camera prefab
pub struct ActiveClipmapPrefab(usize);

impl<'a> PrefabData<'a> for ActiveClipmapPrefab {
    type SystemData = (Write<'a, ActiveClipmap>,);
    type Result = ();

    fn add_to_entity(
        &self,
        _: Entity,
        system_data: &mut Self::SystemData,
        entities: &[Entity],
    ) -> StdResult<(), PrefabError> {
        system_data.0.entity = Some(entities[self.0]);
        // TODO: if no `ActiveClipmap` insert using `LazyUpdate`, require changes to `specs`
        Ok(())
    }
}

pub struct ClipmapSystem;

impl<'a> System<'a> for ClipmapSystem {
    type SystemData = (
        Read<'a, ActiveClipmap>,
        WriteStorage<'a, Clipmap>,
        ReadExpect<'a, Loader>,
        Read<'a, AssetStorage<Mesh>>,
        Read<'a, AssetStorage<Texture>>,

    );

    fn run(&mut self, (active, mut clipmaps, loader, mesh_storage, texture_storage): Self::SystemData) {
        

        // let upload = ShapeUpload{loader: loader, storage: storage,};
        if let Some(active_clipmap) = active.entity {
            let clipmap = clipmaps.get_mut(active_clipmap).unwrap();
            if clipmap.block_mesh.is_none() {
                let block_size = ((clipmap.size + 1)/4) as usize;
                let block_mesh_data = Shape::Plane(Some((block_size - 1, block_size -1 ))).generate::<ComboMeshCreator>(None);
                clipmap.block_mesh = Some(loader.load_from_data(block_mesh_data, (), &mesh_storage));
            }
            // if clipmap.fixup_mesh.is_none() {
            //     let block_size = ((clipmap.size + 1)/4) as usize;
            //     let block_mesh_data = Shape::Plane(Some((block_size, block_size))).generate::<ComboMeshCreator>(None);
            //     clipmap.fixup_mesh = Some(loader.load_from_data(block_mesh_data, (), &storage));
            // }
            if clipmap.elevation.is_none() {
                let heightMetedata = TextureMetadata {
                    sampler: SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile),
                    mip_levels: 1,
                    dynamic: false,
                    format: SurfaceType::R8_G8_B8_A8,
                    size: None,
                    channel: ChannelType::Srgb,
                };
                let elevetion_map_handle =  loader.load(
                    "texture/elevation.png",
                    PngFormat,
                    heightMetedata,
                    (),
                    &texture_storage,
                );
                clipmap.elevation = Some(elevetion_map_handle);
            }
            if clipmap.z_color.is_none() {
                let z_color_handle =  loader.load(
                    "texture/z_color.png",
                    PngFormat,
                    TextureMetadata::srgb(),
                    (),
                    &texture_storage,
                );
                clipmap.z_color = Some(z_color_handle);
            }
            // if clipmap.normal_map.is_none() {
            //     let heightMetedata = TextureMetadata {
            //         sampler: SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile),
            //         mip_levels: 1,
            //         dynamic: true,
            //         format: SurfaceType::R32,
            //         size: None,
            //         channel: ChannelType::Srgb,
            //     };
            //     let normal_map_handle =  loader.load(
            //         "texture/normal_map.png",
            //         PngFormat,
            //         heightMetedata,
            //         (),
            //         &texture_storage,
            //     );
            //     clipmap.normal_map = Some(normal_map_handle);
            // }

        }
    }
}

//     }
pub fn get_clipmap<'a>(
    active: Read<'a, ActiveClipmap>,
    clipmaps: &'a ReadStorage<'a, Clipmap>,
    globals: &'a ReadStorage<'a, GlobalTransform>,
) -> Option<(&'a Clipmap, &'a GlobalTransform)> {
    active
        .entity
        .and_then(|entity| {
            let cm = clipmaps.get(entity);
            let transform = globals.get(entity);
            cm.into_iter().zip(transform.into_iter()).next()
        })
        .or_else(|| (clipmaps, globals).join().next())
}
