pub use self::seperate::DrawClipmap;
pub use self::component::Clipmap;
pub use self::component::ActiveClipmap;
pub use self::component::ClipmapSystem;

mod seperate;
mod component;

static VERT_SRC: &[u8] = include_bytes!("../shaders/vertex/clipmap.glsl");
static FRAG_SRC: &[u8] = include_bytes!("../shaders/fragment/clipmap.glsl");


// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct ClipmapParams {
//     pub scale_factor: f32,
//     pub levels: u32,
// }

// impl Default for ClipmapParams {
//     fn default() -> ClipmapParams {
//         ClipmapParams {
            
//         }
//     }
// }
