use crate::refs::{ObjectReferences, RefType};
use image::{ColorType, DynamicImage};
use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};
use pdf_writer::{Filter, Finish, PdfWriter};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Image(#[from] image::ImageError),
}

#[derive(Debug)]
pub enum RasterImageType {
    DirectlyEmbeddableJpeg(PathBuf),
    Image(DynamicImage),
}

#[derive(Debug)]
pub enum ImageType {
    Raster(RasterImageType),
    SVG(DynamicImage),
}

#[derive(Debug)]
pub struct Image {
    pub image: ImageType,
    pub width: f32,
    pub height: f32,
}

struct EncodeOutput {
    filter: Filter,
    bytes: Vec<u8>,
    mask: Option<Vec<u8>>,
}

impl Image {
    pub fn new_from_disk<P: AsRef<Path>>(path: P) -> Result<Image, ImageError> {
        let path = path.as_ref();
        let is_svg = if let Some(ext) = path.extension() {
            ext.to_ascii_lowercase() == std::ffi::OsString::from("svg")
        } else {
            false
        };

        if is_svg {
            todo!()
        } else {
            Self::new_raster_from_disk(path.to_owned())
        }
    }

    pub fn new_raster_from_disk(path: PathBuf) -> Result<Image, ImageError> {
        let is_tga = if let Some(ext) = path.extension() {
            ext.to_ascii_lowercase() == std::ffi::OsString::from("tga")
        } else {
            false
        };

        let data = std::fs::read(&path)?;

        let format = if is_tga {
            image::ImageFormat::Tga
        } else {
            image::guess_format(&data)?
        };
        let image = image::load_from_memory_with_format(&data, format)?;

        match (format, image.color()) {
            (image::ImageFormat::Jpeg, ColorType::Rgb8) => {
                // we can embed it directly!
                let width = image.width() as f32;
                let height = image.height() as f32;

                Ok(Image {
                    image: ImageType::Raster(RasterImageType::DirectlyEmbeddableJpeg(path)),
                    width,
                    height,
                })
            }
            _ => Self::new_raster(image),
        }
    }

    pub fn new_raster(image: DynamicImage) -> Result<Image, ImageError> {
        let width = image.width() as f32;
        let height = image.height() as f32;
        Ok(Image {
            image: ImageType::Raster(RasterImageType::Image(image)),
            width,
            height,
        })
    }

    fn encode_raster(&self) -> Result<EncodeOutput, ImageError> {
        match &self.image {
            ImageType::Raster(RasterImageType::DirectlyEmbeddableJpeg(path)) => {
                let bytes = std::fs::read(&path)?;
                Ok(EncodeOutput {
                    filter: Filter::DctDecode,
                    bytes,
                    mask: None,
                })
            }
            ImageType::Raster(RasterImageType::Image(image)) => {
                use image::GenericImageView;
                let level = CompressionLevel::DefaultLevel as u8;

                let mask = image.color().has_alpha().then(|| {
                    let alphas: Vec<_> = image.pixels().map(|p| (p.2).0[3]).collect();
                    compress_to_vec_zlib(&alphas, level)
                });

                let bytes = compress_to_vec_zlib(image.to_rgb8().as_raw(), level);

                Ok(EncodeOutput {
                    filter: Filter::FlateDecode,
                    bytes,
                    mask,
                })
            }
            _ => panic!("can't encode SVG as a raster!"),
        }
    }

    pub fn write(
        &self,
        refs: &mut ObjectReferences,
        image_index: usize,
        writer: &mut PdfWriter,
    ) -> Result<(), ImageError> {
        let id = refs
            .get(RefType::Image(image_index))
            .expect("image id exists");

        // TODO: Svg!

        let encoded = self.encode_raster()?;

        let mut image = writer.image_xobject(id, encoded.bytes.as_slice());
        image.filter(encoded.filter);
        image.width(self.width as i32);
        image.height(self.height as i32);
        image.color_space().device_rgb();
        image.bits_per_component(8);

        let mask_id = encoded
            .mask
            .as_ref()
            .map(|_| refs.gen(RefType::ImageMask(image_index)));
        if let Some(mask_id) = &mask_id {
            image.s_mask(mask_id.clone());
        }

        image.finish();

        // add a transparency mask if we have one
        if let Some(mask_id) = mask_id {
            let mut s_mask =
                writer.image_xobject(mask_id, encoded.mask.as_ref().unwrap().as_slice());
            s_mask.width(self.width as i32);
            s_mask.height(self.height as i32);
            s_mask.color_space().device_gray();
            s_mask.bits_per_component(8);
        }

        Ok(())
    }
}
