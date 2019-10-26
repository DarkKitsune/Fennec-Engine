/// Represents a region of tiles in a tile map
#[derive(Copy, Clone, Debug, Default)]
pub struct TileRegion {
    pub top: u32,
    pub left: u32,
    pub width: u32,
    pub height: u32,
    pub center_x: u32,
    pub center_y: u32,
}
