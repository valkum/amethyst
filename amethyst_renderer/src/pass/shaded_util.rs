use gfx::memory::Pod;
use std::mem;

use glsl_layout::*;

use amethyst_core::{
    specs::prelude::{Join, ReadStorage},
    GlobalTransform,
};

use crate::{
    cam::Camera,
    light::Light,
    pipe::{Effect, EffectBuilder},
    resources::AmbientColor,
    types::Encoder,
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Uniform)]
pub struct FragmentArgs {
    point_light_count: uint,
    directional_light_count: uint,
    spot_light_count: uint,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Uniform)]
pub struct PointLightPod {
    position: vec3,
    color: vec3,
    pad: float,
    intensity: float,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Uniform)]
pub struct DirectionalLightPod {
    color: vec3,
    direction: vec3,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Uniform)]
pub struct SpotLightPod {
    position: vec3,
    color: vec3,
    direction: vec3,
    angle: float,
    intensity: float,
    range: float,
    smoothness: float,
}

pub(crate) struct Std140Pod<T: Std140>(T);
unsafe impl<T: Std140> Pod for Std140Pod<T>{}

pub fn set_light_args(
    effect: &mut Effect,
    encoder: &mut Encoder,
    light: &ReadStorage<'_, Light>,
    global: &ReadStorage<'_, GlobalTransform>,
    ambient: &AmbientColor,
    camera: Option<(&Camera, &GlobalTransform)>,
) {
    let point_lights: Vec<_> = (light, global)
        .join()
        .filter_map(|(light, transform)| {
            if let Light::Point(ref light) = *light {
                let position: [f32; 3] = transform.0.column(3).xyz().into();
                Some(
                    Std140Pod(PointLightPod {
                        position: position.into(),
                        color: light.color.into(),
                        intensity: light.intensity,
                        pad: 0.0,
                    }
                    .std140()),
                )
            } else {
                None
            }
        })
        .collect();

    let directional_lights: Vec<_> = light
        .join()
        .filter_map(|light| {
            if let Light::Directional(ref light) = *light {
                Some(
                    Std140Pod(DirectionalLightPod {
                        color: light.color.into(),
                        direction: light.direction.into(),
                    }
                    .std140()),
                )
            } else {
                None
            }
        })
        .collect();

    let spot_lights: Vec<_> = (light, global)
        .join()
        .filter_map(|(light, transform)| {
            if let Light::Spot(ref light) = *light {
                let position: [f32; 3] = transform.0.column(3).xyz().into();
                Some(
                    Std140Pod(SpotLightPod {
                        position: position.into(),
                        color: light.color.into(),
                        direction: light.direction.into(),
                        angle: light.angle.cos(),
                        intensity: light.intensity,
                        range: light.range,
                        smoothness: light.smoothness,
                    }
                    .std140()),
                )
            } else {
                None
            }
        })
        .collect();

    let fragment_args = FragmentArgs {
        point_light_count: point_lights.len() as u32,
        directional_light_count: directional_lights.len() as u32,
        spot_light_count: spot_lights.len() as u32,
    };

    effect.update_constant_buffer("FragmentArgs", &fragment_args.std140(), encoder);
    effect.update_buffer("PointLights", &point_lights[..], encoder);
    effect.update_buffer("DirectionalLights", &directional_lights[..], encoder);
    effect.update_buffer("SpotLights", &spot_lights[..], encoder);

    effect.update_global("ambient_color", Into::<[f32; 3]>::into(*ambient.as_ref()));

    effect.update_global(
        "camera_position",
        camera
            .as_ref()
            .map(|&(_, ref trans)| trans.0.column(3).xyz().into())
            .unwrap_or([0.0; 3]),
    );
}

pub fn setup_light_buffers(builder: &mut EffectBuilder<'_>) {
    builder
        .with_raw_constant_buffer(
            "FragmentArgs",
            mem::size_of::<<FragmentArgs as Uniform>::Std140>(),
            1,
        )
        .with_raw_constant_buffer(
            "PointLights",
            mem::size_of::<<PointLightPod as Uniform>::Std140>(),
            128,
        )
        .with_raw_constant_buffer(
            "DirectionalLights",
            mem::size_of::<<DirectionalLightPod as Uniform>::Std140>(),
            16,
        )
        .with_raw_constant_buffer(
            "SpotLights",
            mem::size_of::<<SpotLightPod as Uniform>::Std140>(),
            128,
        )
        .with_raw_global("ambient_color")
        .with_raw_global("camera_position");
}
