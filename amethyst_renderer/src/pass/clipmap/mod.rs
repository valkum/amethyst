pub use self::interleaved::DrawClipmap;


mod interleaved;

static VERT_SRC: &[u8] = include_bytes!("../shaders/vertex/clipmap.glsl");
static FRAG_SRC: &[u8] = include_bytes!("../shaders/fragment/clipmap.glsl");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipmapParams {
    pub size: i32,
    // pub z_tex_scale_factor: [f32,3],
    // pub one_over_width: [f32,3],
    // pub fine_block_orig: [f32,4],
    // pub z_scale_factor: f32,
    // pub z_tex_scale_factor: f32,
}

impl Default for ClipmapParams {
    fn default() -> ClipmapParams {
        ClipmapParams {
            size: 255,
            // one_over_width: 1/255,
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