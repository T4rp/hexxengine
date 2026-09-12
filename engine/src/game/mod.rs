mod app;
mod context;

pub use app::{GameHandler, start_game_app};
pub use context::GameContext;
use thunderdome::Index;

use crate::text::FontHandle;

// Reserved indices
pub const CUBE_MESH_ID: Index = Index::from_bits(1 << 32 | 0).unwrap();
pub const SPHERE_MESH_ID: Index = Index::from_bits(1 << 32 | 1).unwrap();
pub const CONE_MESH_ID: Index = Index::from_bits(1 << 32 | 2).unwrap();
pub const CUBE_FRAME_MESH_ID: Index = Index::from_bits(1 << 32 | 3).unwrap();

pub const UNIFONT_FONT_ID: Index = Index::from_bits(1 << 32 | 0).unwrap();
pub const NOTOSANS_FONT_ID: Index = Index::from_bits(1 << 32 | 1).unwrap();

pub const UNIFONT_FONT_HANDLE: FontHandle = FontHandle(UNIFONT_FONT_ID);
pub const NOTOSANS_FONT_HANDLE: FontHandle = FontHandle(NOTOSANS_FONT_ID);
