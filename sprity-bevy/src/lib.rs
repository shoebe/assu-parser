use sprity_core::{sheet::SpriteSheet, Loader};

use bevy::{
    asset::{AssetLoader, LoadedAsset},
    math::Vec2,
    prelude::{
        debug, AddAsset, Bundle, Component, GlobalTransform, Handle, Image, Plugin, ResMut,
        Transform,
    },
    reflect::TypeUuid,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{Rect, TextureAtlas},
};

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "442cb6e1-0463-4d41-8e90-3b2c3b0a13a9"]
pub struct SprityAsset {
    pub atlas: Handle<TextureAtlas>,
}

#[derive(Default)]
pub struct SprityAssetLoader {}

impl AssetLoader for SprityAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            debug!("Loading aseprite file: {:?}", load_context.path());
            let loader = sprity_aseprite::binary::loader::BinaryLoader::new();
            let sprite_loader = loader.load_sprite(bytes)?;
            let SpriteSheet { texture, sprites } = sprity_core::sheet::pack(&*sprite_loader)?;
            let width = texture.width();
            let height = texture.height();
            let texture_data = texture.into_raw();
            let texture: Handle<Image> = load_context.set_labeled_asset(
                "texture",
                LoadedAsset::new(Image::new(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    texture_data,
                    TextureFormat::Rgba8Uint,
                )),
            );
            let atlas: Handle<TextureAtlas> = load_context.set_labeled_asset(
                "atlas",
                LoadedAsset::new(TextureAtlas {
                    texture,
                    size: Vec2::new(width as f32, height as f32),
                    textures: sprites
                        .iter()
                        .map(|sprite| Rect {
                            min: Vec2::new(sprite.x as f32, sprite.y as f32),
                            max: Vec2::new(
                                (sprite.x + sprite.width) as f32,
                                (sprite.y + sprite.height) as f32,
                            ),
                        })
                        .collect(),
                    texture_handles: None,
                }),
            );
            load_context.set_default_asset(LoadedAsset::new(SprityAsset { atlas }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ase", "aseprite"]
    }
}

#[derive(Debug, Component)]
pub struct SpritySprite {}

#[derive(Debug, Bundle, Default)]
pub struct SprityBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub sprity_asset: Handle<SprityAsset>,
}

pub struct SprityPlugin;

impl Plugin for SprityPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<SprityAsset>()
            .init_asset_loader::<SprityAssetLoader>();
        // FIXME
    }
}
