use std::ops::{Index, RangeBounds};

use itertools::Itertools;

use crate::{loader::AsepriteFile, make_image::{CroppedImage, Hitbox, LoadImageError}, wrappers::{LayerParameters, TagParameters}};

#[derive(Debug, Clone)]
pub struct ImageId {
    //pub layer_id: usize,
    pub image_ref: String,
    pub offset: (u32, u32),
}

#[derive(Debug, Clone)]
pub struct AnimFrame {
    pub duration: u32, // milliseconds
    pub image_ids: Option<ImageId>, // Todo turn into vec, to have split layers
    pub hitboxes: Vec<Hitbox>,
    pub actions: Vec<()>, // TODO: A frame has no user data, use the ones from each cell all together?
}

#[derive(Debug)]
pub struct Animation {
    pub frames: Vec<AnimFrame>,
    pub actions: TagParameters,
}

#[derive(Debug)]
pub struct AnimationSet {
    pub layer_parameters: Vec<LayerParameters>,
    pub animations: ahash::AHashMap<String, Animation>, // TODO: not string, some form of enum repr?
}

impl AnimationSet {
    pub fn from_ase(file: AsepriteFile<'_>, base_name: &str, packer: &mut texture_packer::MultiTexturePacker<'_, image::RgbaImage, String>) -> anyhow::Result<Self> {
        let mut frame_image_dedup = bimap::BiHashMap::<String,image::RgbaImage,ahash::RandomState, ahash::RandomState>::default();

        let mut anim_frames = Vec::new();
        for (ind, f) in file.frames.into_iter().enumerate() {
            let img = f.combined_frame_image_cropped(&file.layers, &file.images_decompressed);
            let img = match img {
                Ok(img) => Some(img),
                Err(LoadImageError::EmptyFrame) => None,
                Err(e) => anyhow::bail!(e.to_string()),
            };
            let mut img_id = Vec::new();
            if let Some(img) = img {
                let offset = (img.displacement_x, img.displacement_y); 
                let img_ref = if let Some(img_ref) = frame_image_dedup.get_by_right(&img.img) {
                    img_ref.to_owned()
                } else {
                    let img_ref = format!("{base_name}{ind}");
                    // TODO: this packs all frames, even the ones not included under any animations
                    packer.pack_own(img_ref.clone(), img.img.clone()).map_err(|e| anyhow::anyhow!("{e:?}"))?;
                    frame_image_dedup.insert(img_ref.clone(), img.img);
                    img_ref
                };
                img_id.push(ImageId { image_ref: img_ref, offset });
            }

            anim_frames.push(AnimFrame {
                duration: f.duration,
                image_ids: img_id.first().cloned(),
                hitboxes: f.hitboxes(&file.layers, &file.images_decompressed),
                actions: Default::default(),
            })
        }

        let animations: ahash::AHashMap<String, Animation> = file.tags
            .into_iter()
            .map(|t| {
                let frames = anim_frames[t.frame_range()].to_owned();
                
                let a = Animation {
                    frames,
                    actions: t.parameters,
                };
                (t.chunk.name.to_string(), a)
            }).collect();

        let layer_parameters = file.layers.into_iter().map(|l| l.parameters).collect_vec();
        
        Ok(Self {
            layer_parameters,
            animations,
        })
    }
}