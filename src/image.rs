use crate::{
    refs::{ObjectReferences, RefType},
    PDFError,
};
use image::{ColorType, DynamicImage};
use miniz_oxide::deflate::{compress_to_vec_zlib, CompressionLevel};
use pdf_writer::{Filter, Finish, Pdf, Ref};
use std::path::{Path, PathBuf};
use usvg::Tree;

/// A raster image. 24-bit JPEG images may be embedded directly, whereas
/// all other image types are converted to RGB8 and compressed with zlib for
/// embedding. Images with alpha channels have their transparency extracted
/// into a separate soft mask.
///
/// During conversion, the dimensions used in the PDF are taken from the
/// converted RGB8 image rather than the original, ensuring the byte layout
/// matches what PDF readers expect (avoiding row stride misalignment that
/// causes garbled rendering).
pub enum RasterImageType {
    /// A JPEG which may be embedded directly in the file, from disk
    DirectlyEmbeddableJpeg(PathBuf),
    /// A generic image which will be converted to RGB8 when writing the PDF
    Image(DynamicImage),
}

/// Images may be raster images (see [RasterImageType]), or vector images
/// (specifically, SVGs parsed by [usvg](https://crates.io/crates/usvg))
pub enum ImageType {
    /// A raster image
    Raster(RasterImageType),
    /// A parsed SVG
    SVG(Box<Tree>),
}

/// An image with a corresponding width and height. Images may be raster images
/// or vector SVGs. Each image has a corresponding size, which is generally the
/// pixel size of the image. When an image is embedded within the document, the
/// [crate::Page] contents determine the displayed size of the image (in Pt).
pub struct Image {
    /// The image type and cointents
    pub image: ImageType,
    /// The width of the image, nominally in pixels
    pub width: f32,
    /// The height of the image, nominally in pixels
    pub height: f32,
}

impl Image {
    /// Calculate the aspect ratio of the image, returning [f32::INFINITY] if
    /// `[self.height] == 0.0`
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0.0 {
            return f32::INFINITY;
        }
        self.width / self.height
    }
}

struct EncodeOutput {
    filter: Filter,
    bytes: Vec<u8>,
    mask: Option<Vec<u8>>,
    /// actual width of the encoded image data (may differ from original)
    width: u32,
    /// actual height of the encoded image data (may differ from original)
    height: u32,
}

impl Image {
    /// Load an image from a path, automatically determining the format of the image
    /// first from the extension, then other indicators within the file. If the image
    /// cannot be loaded for any reason, will return an error. Note: for all images
    /// (even for JPEGs which may be directly embeddable), this loads the image into
    /// memory at this point (to determine the size of the image) and either stores
    /// it (in most cases), or stores the path where the image can be found which
    /// will be used when rendering the PDF.
    ///
    /// Accepted file types match those from the [image](https://crates.io/crates/image)
    /// crate: PNG, JPEG, GIF, BMP, ICO, TIFF, WebP, AVIF, PNM, DDS, TGA, OpenEXR, farbfeld
    pub fn new_from_disk<P: AsRef<Path>>(path: P) -> Result<Image, PDFError> {
        let path = path.as_ref();
        let is_svg = if let Some(ext) = path.extension() {
            ext.to_ascii_lowercase() == *"svg"
        } else {
            false
        };

        if is_svg {
            Self::new_svg_from_disk(path.to_owned())
        } else {
            Self::new_raster_from_disk(path.to_owned())
        }
    }

    /// Creates a vector image from disk, assuming the file is an `SVG`
    pub fn new_svg_from_disk(path: PathBuf) -> Result<Image, PDFError> {
        let data = std::fs::read(&path)?;
        Self::new_svg(&data)
    }

    /// Creates a vector file from raw bytes, assuming the bytes represent
    /// an `SVG`
    pub fn new_svg(data: &[u8]) -> Result<Image, PDFError> {
        let opts = usvg::Options::default();
        let tree = Tree::from_data(data, &opts)?;
        let size = tree.size();
        let width = size.width();
        let height = size.height();

        Ok(Image {
            image: ImageType::SVG(Box::new(tree)),
            width,
            height,
        })
    }

    /// Creates a raster image from disk, assuming the file is a raster image.
    ///
    /// Accepted file types match those from the [image](https://crates.io/crates/image)
    /// crate: PNG, JPEG, GIF, BMP, ICO, TIFF, WebP, AVIF, PNM, DDS, TGA, OpenEXR, farbfeld
    pub fn new_raster_from_disk(path: PathBuf) -> Result<Image, PDFError> {
        let is_tga = if let Some(ext) = path.extension() {
            ext.to_ascii_lowercase() == *"tga"
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

    /// Creates a raster image from memory, assuming the data represents a raster image.
    ///
    /// Accepted file types match those from the [image](https://crates.io/crates/image)
    /// crate: PNG, JPEG, GIF, BMP, ICO, TIFF, WebP, AVIF, PNM, DDS, TGA, OpenEXR, farbfeld
    pub fn new_raster(image: DynamicImage) -> Result<Image, PDFError> {
        let width = image.width() as f32;
        let height = image.height() as f32;
        Ok(Image {
            image: ImageType::Raster(RasterImageType::Image(image)),
            width,
            height,
        })
    }

    fn encode_raster(&self) -> Result<EncodeOutput, PDFError> {
        match &self.image {
            ImageType::Raster(RasterImageType::DirectlyEmbeddableJpeg(path)) => {
                let bytes = std::fs::read(path)?;
                Ok(EncodeOutput {
                    filter: Filter::DctDecode,
                    bytes,
                    mask: None,
                    width: self.width as u32,
                    height: self.height as u32,
                })
            }
            ImageType::Raster(RasterImageType::Image(image)) => {
                use image::GenericImageView;
                let level = CompressionLevel::DefaultLevel as u8;

                let mask = image.color().has_alpha().then(|| {
                    let alphas: Vec<_> = image.pixels().map(|p| (p.2).0[3]).collect();
                    compress_to_vec_zlib(&alphas, level)
                });

                // use dimensions from the converted RGB8 image, not the original,
                // to ensure the byte layout matches what the PDF reader expects
                let rgb8 = image.to_rgb8();
                let width = rgb8.width();
                let height = rgb8.height();
                let bytes = compress_to_vec_zlib(rgb8.as_raw(), level);

                Ok(EncodeOutput {
                    filter: Filter::FlateDecode,
                    bytes,
                    mask,
                    width,
                    height,
                })
            }
            _ => panic!("can't encode SVG as a raster!"),
        }
    }

    pub(crate) fn write(
        &self,
        refs: &mut ObjectReferences,
        image_index: usize,
        writer: &mut Pdf,
    ) -> Result<(), PDFError> {
        let id = refs.gen(RefType::Image(image_index));

        match &self.image {
            ImageType::Raster(_) => {
                let encoded = self.encode_raster()?;

                let mut image = writer.image_xobject(id, encoded.bytes.as_slice());
                image.filter(encoded.filter);
                image.width(encoded.width as i32);
                image.height(encoded.height as i32);
                image.color_space().device_rgb();
                image.bits_per_component(8);

                let mask_id = encoded
                    .mask
                    .as_ref()
                    .map(|_| refs.gen(RefType::ImageMask(image_index)));
                if let Some(mask_id) = &mask_id {
                    image.s_mask(*mask_id);
                }

                image.finish();

                // add a transparency mask if we have one
                if let Some(mask_id) = mask_id {
                    // unwrap will always be safe as the mask id is mapped from mask to start with
                    let mut s_mask =
                        writer.image_xobject(mask_id, encoded.mask.as_ref().unwrap().as_slice());
                    s_mask.width(encoded.width as i32);
                    s_mask.height(encoded.height as i32);
                    s_mask.color_space().device_gray();
                    s_mask.bits_per_component(8);
                }
            }
            ImageType::SVG(tree) => {
                // convert SVG to a PDF chunk
                let (chunk, svg_ref) =
                    svg2pdf::to_chunk(tree, svg2pdf::ConversionOptions::default())
                        .map_err(|e| PDFError::SvgConversionError(e.to_string()))?;

                // renumber the chunk refs to start from our current ref allocation
                // the `id` we generated should map to the SVG's root XObject ref
                let start_ref = id.get();
                let renumbered = chunk.renumber(|old_ref| {
                    if old_ref == svg_ref {
                        id
                    } else {
                        Ref::new(start_ref + old_ref.get() - svg_ref.get())
                    }
                });

                // update refs to account for all new objects
                let max_ref = renumbered.refs().map(|r| r.get()).max().unwrap_or(id.get());
                refs.set_next_id(Ref::new(max_ref + 1));

                // extend the writer with the SVG chunk
                writer.extend(&renumbered);
            }
        }

        Ok(())
    }
}
