use amethyst_assets::{Handle, PrefabData, PrefabError, Loader, AssetStorage, ProgressCounter, Completion};
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
    pass::{
        shaded_util::{set_light_args, setup_light_buffers},
        util::{get_camera, setup_textures, setup_vertex_args, set_attribute_buffers, set_view_args, ViewArgs},
    },
    resources::AmbientColor,
    ActiveCamera, Camera, Encoder, Factory, Mesh, PosTex, Rgba, Shape, build_mesh_with_combo, ShapeUpload,
    VertexFormat, 
    vertex::{Attributes, Position, Separate, TexCoord},
    MeshData, 
    ComboMeshCreator, MeshCreator,
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
// TODO: Doc
pub struct Clipmap{
    pub initialized: bool,
    pub block_mesh: Option<ClipmapMeshHandle>,
    pub ring_fixup_mesh: Option<ClipmapMeshHandle>,
    pub l_shape_mesh: Option<ClipmapMeshHandle>,
    pub elevation: Option<Handle<Texture>>,
    pub normal: Option<Handle<Texture>>,
    pub z_color: Option<Handle<Texture>>,
    pub size: u32,
    pub alpha_offset: [f32; 2],
    pub one_over_width: [f32; 2],
    

}
impl Clipmap {
    /// Creates a new instance with the default values for all fields
    pub fn new(size: u32) -> Self {
        // Check that size is 2^k-1
        assert!((size + 1) & size == 0);
        let transition_width = size as f32/10.;

        Clipmap {
            block_mesh: None,
            ring_fixup_mesh: None,
            l_shape_mesh: None,
            elevation: None,
            normal: None,
            z_color: None,
            size: size,
            initialized: false,
            // Per forumla this hould be: (n-1)/2-w-1 with w = transition width (n/10)
            alpha_offset: [ ((size as f32 - 1.) / 2. ) - transition_width - 1.; 2],
            // alpha_offset: [transition_width - 1.; 2],
            one_over_width: [1. / transition_width; 2],
        }
    }
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
#[derive(Default)]
pub struct ClipmapSystem {
    progress: Option<ProgressCounter>,
}

impl<'a> System<'a> for ClipmapSystem {
    type SystemData = (
        Read<'a, ActiveClipmap>,
        WriteStorage<'a, Clipmap>,
        ReadExpect<'a, Loader>,
        Read<'a, AssetStorage<Mesh>>,
        Read<'a, AssetStorage<Texture>>,

    );

    fn run(&mut self, (active, mut clipmaps, loader, mesh_storage, texture_storage): Self::SystemData) {
        if let Some(active_clipmap) = active.entity {
            let clipmap = clipmaps.get_mut(active_clipmap).unwrap();
            if let Some(progress) = &self.progress {
                
                match progress.complete() {
                    Completion::Complete => {
                        clipmap.initialized = true;
                        debug!("Clipmap generation completed");
                        self.progress = None;
                    }
                    _ => {
                        dbg!(progress.errors());
                    }
                }
            }
            
            if !clipmap.initialized && self.progress.is_none() {
                debug!("Creating clipmap with size {}x{}", clipmap.size, clipmap.size);
                self.progress = Some(ProgressCounter::default());
                let block_size = ((clipmap.size + 1)/4) as usize;
                let one_offset : f32 = ((clipmap.size+1)/4) as f32 - 1.;
                let half_offset : f32 = one_offset/2.;
                let ring_fixup_offset = 1. + half_offset + one_offset;
                // Generate block mesh with m-1 x m-1 faces (ergo m x m vertices) and scale it by m/2.
                let block_mesh_vert = Shape::Plane(Some((block_size - 1, block_size -1 ))).generate_vertices::<ComboMeshCreator>(Some(((block_size - 1) as f32/2., (block_size - 1) as f32/2., 0.)));
                let block_mesh_data = ComboMeshCreator::from(block_mesh_vert).into();

                clipmap.block_mesh = Some(loader.load_from_data(block_mesh_data, self.progress.as_mut().unwrap(), &mesh_storage));



                let fixup_mesh_horizontal = Shape::Plane(Some((block_size - 1, 2))).generate_vertices::<ComboMeshCreator>(Some(((block_size - 1) as f32/2., 1., 0.)));
                let fixup_mesh_vertical = Shape::Plane(Some((2, block_size - 1))).generate_vertices::<ComboMeshCreator>(Some((1., (block_size - 1) as f32/2., 0.)));

                let mut fixup_mesh_vert_north : Vec<Separate<Position>> = fixup_mesh_vertical.vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0], x[1]-ring_fixup_offset, x[2]])
                    }).collect();
                let mut fixup_mesh_vert_south : Vec<Separate<Position>> = fixup_mesh_vertical.vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0], x[1]+ring_fixup_offset, x[2]])
                    }).collect();

                let mut fixup_mesh_vert_west : Vec<Separate<Position>> = fixup_mesh_horizontal.vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0]-ring_fixup_offset, x[1], x[2]])
                    }).collect();
                let mut fixup_mesh_vert_east : Vec<Separate<Position>> = fixup_mesh_horizontal.vertices()
                    .into_iter()
                    .map(|Separate(x)| {
                        Separate::<Position>::new([x[0]+ring_fixup_offset, x[1], x[2]])
                    }).collect();

                let mut fixup_mesh_vertices : Vec<Separate<Position>> = Vec::new();
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_north);
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_west);
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_east);
                fixup_mesh_vertices.append(&mut fixup_mesh_vert_south);
                let fixup_mesh_data = ComboMeshCreator::from(ComboMeshCreator::new((fixup_mesh_vertices, None, None, None, None))).into();
                
                clipmap.ring_fixup_mesh = Some(loader.load_from_data(fixup_mesh_data, self.progress.as_mut().unwrap(), &mesh_storage));


                let l_shape_mesh_vert = Shape::Plane(Some((block_size - 1, 1))).generate_vertices::<ComboMeshCreator>(Some(((block_size - 1) as f32/2., 1., 0.)));

                let l_shape_mesh_data = ComboMeshCreator::from(l_shape_mesh_vert).into();
                clipmap.l_shape_mesh = Some(loader.load_from_data(l_shape_mesh_data, self.progress.as_mut().unwrap(), &mesh_storage));





                let height_metedata = TextureMetadata {
                    sampler: SamplerInfo::new(FilterMethod::Scale, WrapMode::Tile),
                    mip_levels: 1,
                    dynamic: true,
                    format: SurfaceType::R8_G8_B8_A8,
                    size: None,
                    channel: ChannelType::Srgb,
                };
                // let elevetion_map_handle =  loader.load(
                //     "texture/elevation.png",
                //     PngFormat,
                //     height_metedata,
                //     self.progress.as_mut().unwrap(),
                //     &texture_storage,
                // );
                let elevetion_map_handle =  loader.load(
                    "texture/elevation.png",
                    PngFormat,
                    height_metedata,
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );
                clipmap.elevation = Some(elevetion_map_handle);
                let normal_map_handle =  loader.load(
                    "texture/normal.png",
                    PngFormat,
                    TextureMetadata::unorm(),
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );
                clipmap.normal = Some(normal_map_handle);

                let z_color_handle =  loader.load(
                    "texture/z_color.png",
                    PngFormat,
                    TextureMetadata::srgb(),
                    self.progress.as_mut().unwrap(),
                    &texture_storage,
                );
                clipmap.z_color = Some(z_color_handle);
            }
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