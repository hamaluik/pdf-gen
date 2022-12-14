use thiserror::Error;

/// All errors that the crate can generate
#[derive(Error, Debug)]
pub enum PDFError {
    #[error(transparent)]
    /// An I/O error occurred
    Io(#[from] std::io::Error),

    #[error(transparent)]
    /// [ttf_parser] failed to parse the font
    FaceParsingError(#[from] owned_ttf_parser::FaceParsingError),

    #[error(transparent)]
    /// [image] failed to parse the image
    Image(#[from] image::ImageError),

    #[error(transparent)]
    /// [usvg] failed to parse the image
    Svg(#[from] usvg::Error),

    #[error("The page has not been allocated to the document page arena (the referenced page is missing)")]
    PageMissing,
}
