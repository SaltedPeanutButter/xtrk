use std::path::Path;

use image::{ImageError, ImageFormat, RgbaImage};

use super::Container;

#[derive(Debug, thiserror::Error)]
pub enum ImageIoError {
    #[error("{0}")]
    ImageDecodingError(ImageError),

    #[error("{0}")]
    ImageEncodingError(ImageError),
}

type Result<T> = std::result::Result<T, ImageIoError>;

pub struct Image(RgbaImage);

impl Image {
    /// Open an image from the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Image> {
        let img = image::open(path).map_err(ImageIoError::ImageDecodingError)?;
        Ok(Image(img.to_rgba8()))
    }

    /// Save the image to the given path in PNG format.
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.save(path, ImageFormat::Png)
    }

    /// Save the image to the given path in BMP format.
    pub fn save_bmp<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.save(path, ImageFormat::Bmp)
    }

    /// Save the image to the given path in TIFF format.
    pub fn save_tiff<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.save(path, ImageFormat::Tiff)
    }

    /// Save the image to the given path with the given format.
    fn save<P: AsRef<Path>>(&self, path: P, format: ImageFormat) -> Result<()> {
        self.0
            .save_with_format(path, format)
            .map_err(ImageIoError::ImageEncodingError)?;
        Ok(())
    }
}

impl Container for Image {
    fn as_bytes(&self) -> &[u8] {
        self.0.as_raw()
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}
