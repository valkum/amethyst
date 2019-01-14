pub use self::interleaved::DrawClipmap;

use crate::Attribute;
use gfx_core::format::{ChannelType, Format, SurfaceType};

mod interleaved;

static VERT_SRC: &[u8] = include_bytes!("../shaders/vertex/clipmap.glsl");
static FRAG_SRC: &[u8] = include_bytes!("../shaders/fragment/clipmap.glsl");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipmapParams {
    pub size: usize,
    pub alpha_offset: [f32; 3],
    pub one_over_width: [f32;3],
    // pub fine_block_orig: [f32,4],
}

impl Default for ClipmapParams {
    fn default() -> ClipmapParams {
        let size = 255;
        let transition_width = size/10;
        ClipmapParams {
            size: size,
            // Per forumla this hould be: (n-1)/2-w-1 with w = transition width (n/10)
            alpha_offset: [(size - 1)/2 - transition_width - 1; 3],
            one_over_width: [1 / (size/10); 3],
        }
    }
}


#[derive(Clone, Debug)]
pub enum PosXY {}
impl Attribute for PosXY {
    const NAME: &'static str = "position";
    const FORMAT: Format = Format(SurfaceType::R16_G16, ChannelType::Int);
    const SIZE: u32 = 4;
    type Repr = [u16; 2];
}