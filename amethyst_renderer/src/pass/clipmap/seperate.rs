//! Clipmap pass
//! 
#[allow(unused_imports)]

use amethyst_assets::AssetStorage;
use amethyst_core::{
    nalgebra as na,
    nalgebra::base::coordinates::XYZW,
    nalgebra::Vector3,
    nalgebra::Vector4,
    specs::prelude::{Join, Read, ReadStorage},
    transform::GlobalTransform,
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
        util::{get_camera, setup_textures, setup_vertex_args, set_attribute_buffers, set_vertex_args, ViewArgs},
        clipmap::component::get_clipmap,
    },
    resources::AmbientColor,
    ActiveCamera, Camera, Encoder, Factory, Mesh, PosTex, Rgba, Shape, ComboMeshCreator, build_mesh_with_combo,
    VertexFormat, 
    vertex::{Attributes, Position, TexCoord},
    formats::MeshCreator,
    Texture,
};

use gfx::pso::buffer::ElemStride;
use glsl_layout::{mat4, Uniform, vec4};
use std::mem;
use genmesh as gm;
use genmesh::generators::{SharedVertex, IndexedPolygon};
use std::{
    error::Error as StdError,
    result::Result as StdResult,
};

use super::{FRAG_SRC, VERT_SRC, Clipmap, ActiveClipmap};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Uniform)]
pub(crate) struct VertexArgs {
    proj: mat4,
    view: mat4,
    model: mat4,
    rgba: vec4,
}

static ATTRIBUTES: [Attributes<'static>; 1] = [
    Separate::<Position>::ATTRIBUTES,
];

/// Draw a clipmap
#[derive(Default, Clone, Debug, PartialEq)]
pub struct DrawClipmap {
}

enum TrimOrientation{
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    None
}
// #[derive(Debug)]
// pub enum ClipmapError {
//     SizeError(u32),
//     InvalidBlockID(u32),
//     InvalidLevel(u32)
// }
// impl StdError for ClipmapError {
//     fn description(&self) -> &str {
//         match *self {
//             ClipmapError::SizeError(_) => "Clipmap size is not one less than a power of two!",
//             ClipmapError::InvalidBlockID(_) => "Block ID is not in between [1, 12]!",
//             ClipmapError::InvalidLevel(_) => "Invalid Level!",
//         }
//     }

//     fn cause(&self) -> Option<&dyn StdError> {
//        None,
//     }
// }
// impl Display for ClipmapError {
//     fn fmt(&self, fmt: &mut Formatter<'_>) -> FmtResult {
//         match *self {
//             ClipmapError::SizeError(ref e) => write!(fmt, "Clipmap size is not one less than a power of two: {}", e),
//             ClipmapError::InvalidBlockID(ref e) => write!(fmt, "ck ID is not in between [1, 10]: {}", e),
//             ClipmapError::InvalidLevel(ref e) => write!(fmt, "Invalid Level: {}", e),
//         }
//     }
// }

impl DrawClipmap {
    
    /// Create instance of `DrawClipmap` pass
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns mesh indices and fine-block-origin for given block id
    // TODO: index buffer should be 16-bit for max caching
    // TODO: try to cache this here. Maybe precompute and store as vec in the component.
    // TODO: Change the catchall to unreachable!()
    fn block_offset(
        &mut self, 
        grid_size: u32, 
        texture_size: u32, 
        id: u32, 
        trim_orientation: &TrimOrientation
    ) -> ((f32, f32), (f32, f32)) { 
        let one_offset : f32 = ((grid_size+1)/4) as f32 - 1.;
        let half_offset : f32 = one_offset/2.;
        let trim_offset = match trim_orientation {
            TrimOrientation::NorthEast => (1., 1.),
            TrimOrientation::NorthWest => (0., 2.),
            TrimOrientation::SouthEast => (-2., 0.),
            TrimOrientation::SouthWest => (-1., -1.),
            _ => (0., 0.)
        };

        // Apperently the Shape Generator returns normalized grid coordinates.
        // The resulting Vertex Buffer entries are between [-scale, scale]
        // TODO: Find a way to create mesh with integer grid vertices, for efficient VB Storage
        // let offset: (u32, u32) = match id {
        //   1     => (m-1, 0),
        //   2 | 3 => (id*(m-1) + 2, 0),
        //   4     => (0, m-1),
        //   5     => (3*(m-1) + 2, m-1),
        //   6     => (0, 2*(m-1)+2),
        //   7     => (3*(m-1) + 2, 2*(m-1)+2),
        //   8     => (0, 3*(m-1)+2),
        //   9     => (m-1, 3*(m-1) + 2),
        //   10    => (2*(m-1) + 2, 3*(m-1) + 2),
        //   11    => (3*(m-1) + 2, 3*(m-1) + 2),
        //   _     => (0,0),
        // };
        let offset: (f32, f32) = match id {
            0 => (- 1. - half_offset - one_offset, -1. - half_offset - one_offset),
            1 => (- 1. - half_offset,              -1. - half_offset - one_offset),
            2 => (1. + half_offset,                -1. - half_offset - one_offset),
            3 => (1. + half_offset + one_offset,   -1. - half_offset - one_offset),
            4 => (-1. - half_offset - one_offset,  -1. - half_offset),
            5 => (1. + half_offset + one_offset,   -1. - half_offset),
            6 => (-1. - half_offset - one_offset,  1. + half_offset),
            7 => (1. + half_offset + one_offset,   1. + half_offset),
            8 => (-1. - half_offset - one_offset,  1. + half_offset + one_offset),
            9 => (-1. - half_offset,               1. + half_offset + one_offset),
            10=> (1. + half_offset,                1. + half_offset + one_offset),
            11=> (1. + half_offset + one_offset,   1. + half_offset + one_offset),
            _ => unreachable!()
        };
        // Texture offset is not rel to the center. 
        // We add 1 to the orientation_trim offset to get a value in between [0, size] after adding the offset to each vertex position
        let texture_offset: (f32, f32) = match id {
            0 => (half_offset + trim_offset.0 + 1.,                              half_offset + trim_offset.1 + 1.),
            1 => (half_offset + one_offset + trim_offset.0 + 1.,                 half_offset + trim_offset.1 + 1.),
            2 => (texture_size as f32 - (half_offset + one_offset + trim_offset.0 + 1.), half_offset + trim_offset.1 + 1.),
            3 => (texture_size as f32 - (half_offset + trim_offset.0 + 1.),              half_offset + trim_offset.1 + 1.),
            4 => (half_offset + trim_offset.0 + 1.,                              half_offset + one_offset + trim_offset.1 + 1.),
            5 => (texture_size as f32 - half_offset + trim_offset.0 + 1.,                half_offset + one_offset + trim_offset.1 + 1.),
            6 => (half_offset + trim_offset.0 + 1.,                              texture_size as f32 - (half_offset + one_offset + trim_offset.1 + 1.)),
            7 => (texture_size as f32 - half_offset + trim_offset.0 + 1.,                texture_size as f32 - (half_offset + one_offset + trim_offset.1 + 1.)),
            8 => (half_offset + trim_offset.0 + 1.,                              texture_size as f32 - (half_offset + trim_offset.1 + 1.)),
            9 => (half_offset + one_offset + trim_offset.0 + 1.,                 texture_size as f32 - (half_offset + trim_offset.1 + 1.)),
            10=> (texture_size as f32 - (half_offset + one_offset + trim_offset.0 + 1.), texture_size as f32 - (half_offset + trim_offset.1 + 1.)),
            11=> (texture_size as f32 - (half_offset + trim_offset.0 + 1.),              texture_size as f32 - (half_offset + trim_offset.1 + 1.)),
            _ => unreachable!()
        };
        ((offset.0 + trim_offset.0, offset.1 + trim_offset.1), texture_offset)
    }
    fn draw_block(
        &mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        mesh: &Mesh,
        size: u32,
        texture_size: u32,
        one_over_texture: f32,
        level: u32,
        id: u32,
        trim_orientation: &TrimOrientation
        ) 
    {
        let scale = (1 << (level)) as f32;
        let (offset, texture_offset) = self.block_offset(size, texture_size, id, &trim_orientation);
        effect.update_global("scale_factor", Into::<[f32; 4]>::into([ scale, scale, offset.0, offset.1]));
        effect.update_global("fine_block_orig", Into::<[f32; 4]>::into([one_over_texture, one_over_texture, texture_offset.0, texture_offset.1]));
    
        effect.draw(mesh.slice(), encoder);
    }
    fn draw_l(
        &mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        mesh: &Mesh,
        size: u32,
        texture_size: u32,
        one_over_texture: f32,
        level: u32,
        trim_orientation: &TrimOrientation
    ) {
        let trim_offset = match trim_orientation {
            TrimOrientation::NorthEast => (-1., -1.),
            TrimOrientation::NorthWest => (0., 2.),
            TrimOrientation::SouthEast => (-2., 0.),
            TrimOrientation::SouthWest => (-1., -1.),
            _ => (0., 0.)
        };
        let scale = (1 << (level)) as f32;
        let offset = trim_offset;
        effect.update_global("scale_factor", Into::<[f32; 4]>::into([ scale, scale, offset.0, offset.1]));
        effect.update_global("fine_block_orig", Into::<[f32; 4]>::into([one_over_texture, one_over_texture, 0.0, 0.]));
        effect.draw(mesh.slice(), encoder);
    }
    fn draw_fixup(
        &mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        mesh: &Mesh,
        size: u32,
        texture_size: u32,
        one_over_texture: f32,
        level: u32,
        trim_orientation: &TrimOrientation
    ){
        let trim_offset = match trim_orientation {
            TrimOrientation::NorthEast => (-1., -1.),
            TrimOrientation::NorthWest => (0., 2.),
            TrimOrientation::SouthEast => (-2., 0.),
            TrimOrientation::SouthWest => (-1., -1.),
            _ => (0., 0.)
        };
        let scale = (1 << (level)) as f32;
        let offset = trim_offset;
        effect.update_global("scale_factor", Into::<[f32; 4]>::into([ scale, scale, offset.0, offset.1]));
        effect.update_global("fine_block_orig", Into::<[f32; 4]>::into([one_over_texture, one_over_texture, 0., 0.]));
        effect.draw(mesh.slice(), encoder);
    }
    /// Draws a clipmap layer.
    // TODO: change the textures here, as each level has its own
    fn draw_layer(&mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        block_mesh: Option<&Mesh>,
        fixup_mesh: Option<&Mesh>,
        l_mesh: Option<&Mesh>,
        size: u32,
        texture_size: u32,
        one_over_texture: f32,
        level: u32,
        trim_orientation: TrimOrientation
    ) {
        effect.update_global("color_overwrite", Into::<[f32; 4]>::into([1.0, 0.0, 0.0, 1.0]));
        if let Some(mesh) = block_mesh {
            // TODO: Figure out if this is slower than drawing all blocks for all layer first and then all other shapes respectively
            if !set_attribute_buffers(effect, mesh, &ATTRIBUTES)
            {
                effect.clear();
                error!("Could not set attribute buffer");
                return;
            }
            for id in 0..12 {
                self.draw_block(encoder, effect, mesh, size, texture_size, one_over_texture, level, id, &trim_orientation);
            }
        }
        effect.update_global("color_overwrite", Into::<[f32; 4]>::into([0.0, 0.5, 1.0, 1.0]));
        if let Some(mesh) = fixup_mesh {
            if !set_attribute_buffers(effect, mesh, &ATTRIBUTES)
            {
                effect.clear();
                error!("Could not set attribute buffer");
                return;
            }
            dbg!(&mesh);
            self.draw_fixup(encoder, effect, mesh, size, texture_size, one_over_texture, level, &trim_orientation);
        }
        // effect.update_global("color_overwrite", Into::<[f32; 4]>::into([0.0, 1.0, 0.0, 1.0]));
        // if let Some(mesh) = l_mesh {
        //     if !set_attribute_buffers(effect, mesh, &ATTRIBUTES)
        //     {
        //         effect.clear();
        //         error!("Could not set attribute buffer");
        //         return;
        //     }
        //     self.draw_l(encoder, effect, mesh, size, texture_size, one_over_texture, level, &trim_orientation);
        // }

    }
}

impl<'a> PassData<'a> for DrawClipmap
{
    type Data = (
        Read<'a, ActiveCamera>,
        ReadStorage<'a, Camera>,
        Read<'a, AssetStorage<Mesh>>,
        Read<'a, AmbientColor>,
        ReadStorage<'a, GlobalTransform>,
        ReadStorage<'a, Light>,
        Read<'a, ActiveClipmap>,
        ReadStorage<'a, Clipmap>,
        Read<'a, AssetStorage<Texture>>,
    );
}

impl Pass for DrawClipmap {
    fn compile(&mut self, mut effect: NewEffect<'_>) -> Result<Effect> {
        let mut builder = effect.simple(VERT_SRC, FRAG_SRC);


        builder.without_back_face_culling();
        builder.with_raw_constant_buffer(
            "VertexArgs",
            mem::size_of::<<VertexArgs as Uniform>::Std140>(),
            1,
        );
        setup_light_buffers(&mut builder);
        builder
            // TODO: keep this vertex buffer filled with the block mesh
            // TODO: add vertex buffer for fixup and trim
            .with_raw_vertex_buffer(Separate::<Position>::ATTRIBUTES, Separate::<Position>::size() as ElemStride, 0)
            .with_texture("elevation_sampler")
            .with_texture("normal_sampler")
            .with_texture("z_based_color")
            .with_raw_global("size")
            .with_raw_global("z_scale_factor")
            .with_raw_global("z_tex_scale_factor")
            .with_raw_global("alpha_offset")
            .with_raw_global("one_over_width")
            // .with_raw_global("camera_position")
            .with_raw_global("fine_block_orig")
            .with_raw_global("scale_factor")
            .with_raw_global("color_overwrite")
            .with_output("color", Some(DepthMode::LessEqualWrite))
            .build()
    }

    fn apply<'a, 'b: 'a>(
        &'a mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        mut _factory: Factory,
        (active, camera, mesh_storage, ambient, globals, lights, active_clipmap, clipmaps, textures): <Self as PassData<'a>>::Data,
    ) {
        let camera = get_camera(active, &camera, &globals);
        
        set_light_args(effect, encoder, &lights, &globals, &ambient, camera);
        
        if let Some((clipmap, global)) = get_clipmap(active_clipmap, &clipmaps, &globals)
        {
            set_vertex_args(
                effect,
                encoder,
                camera,
                &global,
                Rgba::WHITE,
            );
            if clipmap.initialized {
                // let block_mesh_handle = clipmap.get_block_mesh();
                // if block_mesh_handle.initiali() { return; }
                // let block_mesh = mesh_storage.get(block_mesh_handle.unwrap()).expect("Mesh not in Storage");
                let block_mesh = mesh_storage.get(&clipmap.block_mesh.as_ref().unwrap());
                let ring_fixup_mesh = mesh_storage.get(&clipmap.ring_fixup_mesh.as_ref().unwrap());
                let l_shape_mesh = mesh_storage.get(&clipmap.l_shape_mesh.as_ref().unwrap());
                // fine_block_orig.xy: 1/(w, h) of texture
                // fine_block_orig.zw: origin of block in texture
                let mut texture_size = 0;
                let mut one_over_texture = 1.;
                if let Some(elevation_texture) = textures.get(&clipmap.elevation.as_ref().unwrap()) {
                    effect.data.textures.push(elevation_texture.view().clone());
                    effect.data.samplers.push(elevation_texture.sampler().clone());
                    one_over_texture = 1. / elevation_texture.size().0 as f32;
                    texture_size = elevation_texture.size().0 as u32;
                }

                if let Some(normal_texture) = textures.get(&clipmap.normal.as_ref().unwrap()) { 
                    effect.data.textures.push(normal_texture.view().clone());
                    effect.data.samplers.push(normal_texture.sampler().clone());
                }

                
                if let Some(z_color_texture) = textures.get(&clipmap.z_color.as_ref().unwrap()) { 
                    effect.data.textures.push(z_color_texture.view().clone());
                    effect.data.samplers.push(z_color_texture.sampler().clone());
                }
                effect.update_global("size", Into::<i32>::into(clipmap.size as i32));

                let z_scale_factor = 255.0;
                effect.update_global("z_scale_factor", Into::<f32>::into(z_scale_factor));
                let z_tex_scale_factor = 100.;
                effect.update_global("z_tex_scale_factor", Into::<f32>::into(z_tex_scale_factor));

                // Per forumla this hould be: (n-1)/2-w-1 with w = transition width (n/10)
                effect.update_global("alpha_offset", Into::<[f32; 2]>::into(clipmap.alpha_offset));
                effect.update_global("one_over_width", Into::<[f32; 2]>::into(clipmap.one_over_width));
                // let player_camera_pos = camera
                //     .as_ref()
                //     .map(|&(ref cam, ref transform)| {
                //         let view: [f32; 3] = transform.0.column(3).xyz().into();
                //         view
                //     })
                //     .unwrap_or_else(|| {
                //         let identity: [f32; 3] = Vector3::new(0., 0., 0.).into();
                //         identity
                //     });
                // effect.update_global("camera_position", Into::<[f32; 3]>::into(player_camera_pos));

                

                // Scale_factor.xy: grid spacing of current level
                // Scale_factor.zw: origin of current block within world 
                // let spacing = 1.;
                // let mut scale_factor = [100., 100., 0., 0.];
                // effect.update_global("scale_factor", Into::<[f32; 4]>::into(scale_factor));


                // effect.draw(block_mesh.slice(), encoder);
                self.draw_layer(encoder, effect, block_mesh, ring_fixup_mesh, l_shape_mesh, clipmap.size, texture_size, one_over_texture, 0, TrimOrientation::NorthEast);
                // self.draw_layer(encoder, effect, block_mesh, l_shape_mesh, ring_fixup_mesh, clipmap.size, texture_size, one_over_texture, 1, TrimOrientation::NorthEast);
                // self.draw_layer(encoder, effect, block_mesh, l_shape_mesh, ring_fixup_mesh, clipmap.size, texture_size, one_over_texture, 2, TrimOrientation::NorthEast);
                // for block_id in 0..12 {
                //     self.draw_block(encoder, effect, block_mesh, clipmap.size, spacing, texture_size, one_over_texture, 5, block_id, TrimOrientation::SouthWest);    
                // }
                // for block_id in 0..12 {
                //     self.draw_block(encoder, effect, block_mesh, clipmap.size, 2.*spacing, texture_size, one_over_texture, 4, block_id, TrimOrientation::None);    
                // }
                // for block_id in 0..12 {
                //     self.draw_block(encoder, effect, block_mesh, clipmap.size, 4.*spacing, texture_size, one_over_texture, 3, block_id, TrimOrientation::None);    
                // }
                // for block_id in 0..12 {
                //     self.draw_block(encoder, effect, block_mesh, clipmap.size, 8.*spacing, texture_size, one_over_texture, 2, block_id, TrimOrientation::NorthEast);    
                // }
                // for block_id in 0..12 {
                //     self.draw_block(encoder, effect, block_mesh, clipmap.size, 16.*spacing, texture_size, one_over_texture, 1, block_id, TrimOrientation::SouthEast);    
                // }
                // for block_id in 0..12 {
                //     self.draw_block(encoder, effect, block_mesh, clipmap.size, 32.*spacing, texture_size, one_over_texture, 0, block_id, TrimOrientation::SouthWest);    
                // }
            }


            effect.clear();
        }
    }
    
}
