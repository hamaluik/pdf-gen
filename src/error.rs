use thiserror::Error;

/// All errors that the crate can generate
#[derive(Error, Debug)]
pub enum PDFError {
    #[error(transparent)]
    /// An I/O error occurred
    Io(#[from] std::io::Error),

    #[error(transparent)]
    /// Font parsing failed (via `owned_ttf_parser`)
    FaceParsingError(#[from] owned_ttf_parser::FaceParsingError),

    #[error(transparent)]
    /// Image parsing failed (via the `image` crate)
    Image(#[from] image::ImageError),

    #[error(transparent)]
    /// SVG parsing failed (via `usvg`)
    Svg(#[from] usvg::Error),

    #[error("SVG conversion error: {0}")]
    /// SVG to PDF conversion failed (via `svg2pdf`)
    SvgConversionError(String),

    #[error("The page has not been allocated to the document page arena (the referenced page is missing)")]
    PageMissing,
}
