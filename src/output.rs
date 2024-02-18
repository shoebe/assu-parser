use std::ops::{Index, RangeBounds};

use itertools::Itertools;

use crate::{loader::AsepriteFile, make_image::{CroppedImage, Hitbox, LoadImageError}, wrappers::{LayerParameters, TagParameters}};

#[derive(Debug, Clone)]
pub struct ImageId {
    //pub layer_id: usize,
    pub image_ref: String,
    // offset by this amount if texture has tl as origin
    pub tl_offset: (u32, u32),
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
    pub canvas_size: (u32, u32),
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
                let tl_offset = (img.displacement_x, img.displacement_y); 
                let img_ref = if let Some(img_ref) = frame_image_dedup.get_by_right(&img.img) {
                    img_ref.to_owned()
                } else {
                    let img_ref = format!("{base_name}{ind}");
                    // TODO: this packs all frames, even the ones not included under any animations
                    packer.pack_own(img_ref.clone(), img.img.clone()).map_err(|e| anyhow::anyhow!("{e:?}"))?;
                    frame_image_dedup.insert(img_ref.clone(), img.img);
                    img_ref
                };

                img_id.push(ImageId { image_ref: img_ref, tl_offset });
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
            canvas_size: (file.header.width as u32, file.header.height as u32),
            layer_parameters,
            animations,
        })
    }
}

pub fn tl_offset_to_centered(tl_offset: (u32, u32), sprite_size: (u32, u32), canvas_size: (u32, u32)) -> (f32, f32){
    // Motivation: do not want the 'centered' sprite to be offset by 1/2 a pixel compared to other sprites
    //             This happens if the canvas has even dimensions, the resulting sprite will be too centered

    // Ex Sprite: (x, y, w, h) = (3, 3, 13, 13)
    // canvas: 17x17
    // canvas center= (8.5,8.5)
    // center of the sprite: (13,13)/2 + (3,3) = (9.5, 9.5)
    // center offset = (8.5,8.5) - (9.5,9.5) = (-1,-1)
    // center offset in action = (-1,-1) - (13,13)/2
    //                         = (-7.5,-7.5) half a pixel off

    // THEREFORE: add a (+1,+1) offset to canvas center when canvas size is odd

    let canvas_center = ((canvas_size.0 + canvas_size.0 % 2) as f32 / 2.0, (canvas_size.1 + canvas_size.1 % 2) as f32 / 2.0);

    let local_sprite_center = (sprite_size.0 as f32 / 2.0,sprite_size.1 as f32 / 2.0);

    let sprite_center_wrt_canvas = (tl_offset.0 as f32 + local_sprite_center.0 ,tl_offset.1 as f32 + local_sprite_center.1);

    (canvas_center.0 - sprite_center_wrt_canvas.0, canvas_center.1 - sprite_center_wrt_canvas.1)
}

#[cfg(test)]
mod tests {
    use crate::output::tl_offset_to_centered;

    #[test]
    fn it_has_no_half_pixel_vertices() {
        for (c_w, c_h, s_w, s_h) in [
            (22, 22, 16, 16), // even even
            (22, 22, 17, 17), // even odd
            (21, 21, 17, 17), // odd odd
            (21, 21, 16, 16), // odd even
        ] {
            let off = tl_offset_to_centered((0,0), (s_w, s_h), (c_w, c_h));
            let left_top_pos = (off.0 + s_w as f32/2.0, off.1 + s_h as f32 / 2.0);
            
            // should never have 0.5 offsets, no matter the even/odd combo of sprite/canvas
            assert!(left_top_pos.0.fract() < 0.00001);
            assert!(left_top_pos.1.fract() < 0.00001);
        }
    }
}
