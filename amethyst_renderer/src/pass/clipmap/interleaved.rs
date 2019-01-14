//! Skybox pass

use amethyst_core::{
    nalgebra as na,
    specs::{Read, ReadStorage},
    transform::GlobalTransform,
};

use crate::{
    error::Result,
    pipe::{
        pass::{Pass, PassData},
        DepthMode, Effect, NewEffect,
    },
    pass::{
        shaded_util::{set_light_args, setup_light_buffers},
        util::{get_camera, setup_textures, setup_vertex_args},
    },
    set_vertex_args, ActiveCamera, Camera, Encoder, Factory, Mesh, PosTex, Rgba, Shape,
    VertexFormat,
    vertex::Attributes,
};

use gfx::pso::buffer::ElemStride;
use glsl_layout::{mat4, Uniform, vec4};

use super::{FRAG_SRC, VERT_SRC, ClipmapParams, PosXY};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Uniform)]
pub(crate) struct VertexArgs {
    proj: mat4,
    view: mat4,
    model: mat4,
    rgba: vec4,
}

/// Draw a clipmap
#[derive(Clone, Debug)]
pub struct DrawClipmap {
    mesh: Option<Mesh>,
}

enum TrimOrientation{
    North,
    East,
    South,
    West,

}

impl DrawClipmap {
    /// Create instance of `DrawClipmap` pass
    pub fn new() -> Self {
        DrawClipmap { mesh: None }
    }
    /// Returns mesh indices and fine-block-origin for given block id
    /// TODO: Cache this somewhere
    /// index buffer should be 16-bit for max caching
    fn block(id: usize, size: usize) -> (Slice<Resources>, [f32; 2]) {
        // 64
        // let m = (size+1)/4;
        // let offset = id - 1;
        // let indices = Vec::with_capacity(64);
        // for y in 0..size-1 {
        //     for k in (y*size)..(y*size+2) {
        //         indices.push(k+offset, k+1+offset, k+15+offset));
        //         push(tri(k+1, k+15, k+16));
        //     }
        // }

    }
    fn ring_fix_up(orient: TrimOrientation, size: usize) {


    }
}

impl<'a> PassData<'a> for DrawClipmap {
    type Data = (
        Read<'a, ActiveCamera>,
        ReadStorage<'a, Camera>,
        ReadStorage<'a, GlobalTransform>,
        Read<'a, ClipmapParams>,
    );
}

impl Pass for DrawClipmap {
    fn compile(&mut self, mut effect: NewEffect<'_>) -> Result<Effect> {
        let mut builder = effect.simple(VERT_SRC, FRAG_SRC);

        builder.without_back_face_culling();
        setup_vertex_args(&mut builder);
        setup_light_buffers(&mut builder);
        setup_textures(&mut builder, &TEXTURES);
        builder
            .with_raw_vertex_buffer(PosXY::ATTRIBUTES, PosXY::size() as ElemStride, 0)
            .with_texture("elevation_sampler")
            .with_raw_global("camera_position")
            .with_raw_global("alpha_offset")
            .with_raw_global("one_over_width")
            .with_raw_global("fine_block_orig")
            .with_raw_global("scale_factor")
            .with_raw_global("z_scale_factor")
            .with_raw_global("z_tex_scale_factor")
            .with_raw_global("size")
            .build()
    }

    fn apply<'a, 'b: 'a>(
        &'a mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        mut _factory: Factory,
        (active, camera, global, clipmap_params): <Self as PassData<'a>>::Data,
    ) {
        let camera = get_camera(active, &camera, &global);


        set_vertex_args(
            effect,
            encoder,
            camera,
            &GlobalTransform(na::one()),
            Rgba::WHITE,
        );
        effect.update_global("size", Into::<i32>::into(clipmap_params.size));

        // Per forumla this hould be: (n-1)/2-w-1 with w = transition width (n/10)
        effect.update_global("alpha_offset", Into::<[f32; 3]>::into(clipmap_params.alpha_offset));
        effect.update_global("one_over_width", Into::<[f32; 3]>::into(clipmap_params.one_over_width));

        // fine_block_orig.xy: 1/(w, h) of texture
        // fine_block_orig.zw: origin of block in texture
        let 1_over_texture = 1/
        let fine_block_orig = [1_over_texture, 1_over_texture, ];
        effect.update_global("fine_block_orig", Into::<[f32; 3]>::into(fine_block_orig));
        let scale_factor = [1; 4];
        effect.update_global("scale_factor", Into::<[f32; 4]>::into(scale_factor));
        let z_scale_factor = [1; 3];
        effect.update_global("z_scale_factor", Into::<[f32; 3]>::into(z_scale_factor));
        let z_tex_scale_factor = [1; 3];
        effect.update_global("z_tex_scale_factor", Into::<[f32; 3]>::into(z_tex_scale_factor));



        effect.draw(mesh.slice(), encoder);
        effect.clear();
    }
}
