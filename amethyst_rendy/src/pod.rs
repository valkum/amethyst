use crate::{
    mtl,
    resources::Tint,
    sprite::{SpriteRender, SpriteSheet},
    types::Texture,
};
use amethyst_assets::{AssetStorage, Handle};
use amethyst_core::{math::Vector4, GlobalTransform};
use glsl_layout::*;
use rendy::{
    hal::{format::Format, Backend},
    mesh::{AsVertex, Attribute, VertexFormat},
};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, AsStd140)]
#[repr(C, align(16))]
pub(crate) struct TextureOffset {
    pub u_offset: vec2,
    pub v_offset: vec2,
}

impl TextureOffset {
    pub(crate) fn from_offset(offset: &crate::mtl::TextureOffset) -> Self {
        TextureOffset {
            u_offset: [offset.u.0, offset.u.1].into(),
            v_offset: [offset.v.0, offset.v.1].into(),
        }
    }
}

#[derive(Clone, Copy, Debug, AsStd140)]
#[repr(C, align(16))]
pub(crate) struct ViewArgs {
    pub proj: mat4,
    pub view: mat4,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(16))]
pub(crate) struct VertexArgs {
    pub model: mat4,
    pub tint: vec4,
}

impl VertexArgs {
    #[inline]
    pub fn from_object_data(object: &GlobalTransform, tint: Option<&Tint>) -> Self {
        let model: [[f32; 4]; 4] = (*object).into();
        VertexArgs {
            model: model.into(),
            tint: tint.map_or([1.0; 4].into(), |t| {
                let (r, g, b, a) = t.0.into_components();
                [r, g, b, a].into()
            }),
        }
    }
}

impl AsVertex for VertexArgs {
    const VERTEX: VertexFormat<'static> = VertexFormat {
        attributes: Cow::Borrowed(&[
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 0,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 16,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 32,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 48,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 64,
            },
        ]),
        stride: 80,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct SkinnedVertexArgs {
    pub model: mat4,
    pub tint: vec4,
    pub joints_offset: uint,
}

impl SkinnedVertexArgs {
    #[inline]
    pub fn from_object_data(
        object: &GlobalTransform,
        tint: Option<&Tint>,
        joints_offset: u32,
    ) -> Self {
        let model: [[f32; 4]; 4] = (*object).into();
        SkinnedVertexArgs {
            model: model.into(),
            tint: tint.map_or([1.0; 4].into(), |t| {
                let (r, g, b, a) = t.0.into_components();
                [r, g, b, a].into()
            }),
            joints_offset,
        }
    }
}

impl AsVertex for SkinnedVertexArgs {
    const VERTEX: VertexFormat<'static> = VertexFormat {
        attributes: Cow::Borrowed(&[
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 0,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 16,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 32,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 48,
            },
            Attribute {
                format: Format::Rgba32Sfloat,
                offset: 64,
            },
            Attribute {
                format: Format::R32Uint,
                offset: 80,
            },
        ]),
        stride: 84,
    };
}

#[derive(Clone, Copy, Debug, AsStd140)]
pub(crate) struct PointLight {
    pub position: vec3,
    pub color: vec3,
    pub intensity: float,
}

#[derive(Clone, Copy, Debug, AsStd140)]
pub(crate) struct DirectionalLight {
    pub color: vec3,
    pub direction: vec3,
}

#[derive(Clone, Copy, Debug, AsStd140)]
pub(crate) struct SpotLight {
    pub position: vec3,
    pub color: vec3,
    pub direction: vec3,
    pub angle: float,
    pub intensity: float,
    pub range: float,
    pub smoothness: float,
}

#[derive(Clone, Copy, Debug, AsStd140)]
pub(crate) struct Environment {
    pub ambient_color: vec3,
    pub camera_position: vec3,
    pub point_light_count: int,
    pub directional_light_count: int,
    pub spot_light_count: int,
}

#[derive(Clone, Copy, Debug, AsStd140)]
#[repr(C, align(16))]
pub(crate) struct Material {
    pub alpha_cutoff: float,
    pub(crate) uv_offset: TextureOffset,
}

impl Material {
    pub fn from_material<B: Backend>(mat: &mtl::Material<B>) -> Self {
        Material {
            alpha_cutoff: mat.alpha_cutoff,
            uv_offset: TextureOffset::from_offset(&mat.uv_offset),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct SpriteArgs {
    pub dir_x: vec2,
    pub dir_y: vec2,
    pub pos: vec2,
    pub u_offset: vec2,
    pub v_offset: vec2,
    pub depth: float,
}

impl SpriteArgs {
    pub fn from_data<'a, B: Backend>(
        tex_storage: &AssetStorage<Texture<B>>,
        sprite_storage: &'a AssetStorage<SpriteSheet<B>>,
        sprite_render: &SpriteRender<B>,
        global_transform: &GlobalTransform,
    ) -> Option<(Self, &'a Handle<Texture<B>>)> {
        let sprite_sheet = sprite_storage.get(&sprite_render.sprite_sheet)?;
        if !tex_storage.contains(&sprite_sheet.texture) {
            return None;
        }

        let sprite = &sprite_sheet.sprites[sprite_render.sprite_number];

        let transform = &global_transform.0;
        let dir_x = transform.column(0) * sprite.width;
        let dir_y = transform.column(1) * -sprite.height;
        let pos = transform * Vector4::new(-sprite.offsets[0], -sprite.offsets[1], 0.0, 1.0);

        Some((
            SpriteArgs {
                dir_x: dir_x.xy().into_pod(),
                dir_y: dir_y.xy().into_pod(),
                pos: pos.xy().into_pod(),
                u_offset: [sprite.tex_coords.left, sprite.tex_coords.right].into(),
                v_offset: [sprite.tex_coords.bottom, sprite.tex_coords.top].into(),
                depth: pos.z,
            },
            &sprite_sheet.texture,
        ))
    }
}

impl AsVertex for SpriteArgs {
    const VERTEX: VertexFormat<'static> = VertexFormat {
        attributes: Cow::Borrowed(&[
            Attribute {
                format: Format::Rg32Sfloat,
                offset: 0,
            },
            Attribute {
                format: Format::Rg32Sfloat,
                offset: 8,
            },
            Attribute {
                format: Format::Rg32Sfloat,
                offset: 16,
            },
            Attribute {
                format: Format::Rg32Sfloat,
                offset: 24,
            },
            Attribute {
                format: Format::Rg32Sfloat,
                offset: 32,
            },
            Attribute {
                format: Format::R32Sfloat,
                offset: 40,
            },
        ]),
        stride: 44,
    };
}

pub trait IntoPod<T> {
    fn into_pod(self) -> T;
}

impl IntoPod<vec2> for amethyst_core::math::Vector2<f32> {
    fn into_pod(self) -> vec2 {
        let arr: [f32; 2] = self.into();
        arr.into()
    }
}

impl IntoPod<vec3> for amethyst_core::math::Vector3<f32> {
    fn into_pod(self) -> vec3 {
        let arr: [f32; 3] = self.into();
        arr.into()
    }
}

impl IntoPod<vec3> for palette::Srgb {
    fn into_pod(self) -> vec3 {
        let (r, g, b) = self.into_components();
        [r, g, b].into()
    }
}

impl IntoPod<vec4> for palette::Srgba {
    fn into_pod(self) -> vec4 {
        let (r, g, b, a) = self.into_components();
        [r, g, b, a].into()
    }
}
