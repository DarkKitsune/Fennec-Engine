use super::tileregion::TileRegion;
use crate::error::FennecError;

/// A layer for sprites
pub struct SpriteLayer {
    highest_sprite: Option<usize>,
    sprite_count: usize,
    sprites: [Option<Sprite>; Self::MAX_SPRITES],
}

impl SpriteLayer {
    /// The maximum number of sprites allowed in a sprite layer
    pub const MAX_SPRITES: usize = 65536;

    /// Factory method
    pub fn new() -> Self {
        Self {
            highest_sprite: None,
            sprite_count: 0,
            sprites: [None; Self::MAX_SPRITES],
        }
    }

    /// Adds a new sprite to the layer and returns the new sprite's handle
    pub fn create(
        &mut self,
        position: (f32, f32),
        tile_region: TileRegion,
    ) -> Result<SpriteHandle, FennecError> {
        let index = self.first_empty().ok_or_else(|| {
            FennecError::new(format!(
                "The max number of sprites ({}) has been reached",
                Self::MAX_SPRITES
            ))
        })?;
        if self.highest_sprite.is_none() || index > self.highest_sprite.unwrap() {
            self.highest_sprite = Some(index);
        }
        self.sprite_count += 1;
        self.sprites[index] = Some(Sprite::new(position, tile_region));
        Ok(SpriteHandle { array_index: index })
    }

    /// Removes the sprite pointed to by the given handle from the sprite layer
    pub fn destroy(&mut self, handle: SpriteHandle) -> Result<(), FennecError> {
        if self.sprites[handle.array_index].is_none() {
            return Err(FennecError::new(format!(
                "No sprite exists with handle: {:?}",
                handle
            )));
        }
        self.sprites[handle.array_index] = None;
        self.sprite_count -= 1;
        if handle.array_index == self.highest_sprite.unwrap() {
            if self.sprite_count == 0 {
                self.highest_sprite = None;
            } else {
                for idx in (self.highest_sprite.unwrap() - 1)..=0 {
                    if self.sprites[idx].is_some() {
                        self.highest_sprite = Some(idx);
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Finds the first empty sprite index
    fn first_empty(&self) -> Option<usize> {
        if self.sprite_count == Self::MAX_SPRITES {
            return None;
        }
        if let Some(highest_sprite) = self.highest_sprite {
            for (idx, sprite) in self.sprites.iter().take(highest_sprite).enumerate() {
                if sprite.is_none() {
                    return Some(idx);
                }
            }
            Some(highest_sprite + 1)
        } else {
            Some(0)
        }
    }
}

/// A single sprite object in a SpriteLayer
#[derive(Copy, Clone, Debug)]
struct Sprite {
    position: (f32, f32),
    tile_region: TileRegion,
}

impl Sprite {
    /// Factory method
    fn new(position: (f32, f32), tile_region: TileRegion) -> Sprite {
        Self {
            position,
            tile_region,
        }
    }
}

/// A handle pointing to a sprite in a sprite layer
#[derive(Clone, Debug, Hash)]
pub struct SpriteHandle {
    array_index: usize,
}
