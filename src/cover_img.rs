use egui::{ColorImage, TextureHandle};
use image::GenericImageView;

#[derive(Default)]
pub struct CoverImg {
    img_data: Option<(Vec<u8>, [usize; 2])>,
    texture: Option<TextureHandle>,
}

impl CoverImg {
    pub fn load_image_data(&mut self, path: &str) {
        if let Some(texture) = self.texture.take() {
            // Explicitly drop the texture handle to deallocate the old texture if exists
            drop(texture);
            log::info!("Dropped old texture");
        }

        if let Ok(image) = image::open(path) {
            let (width, height) = image.dimensions();
            let image_data = image.to_rgba8().into_raw();
            self.img_data = Some((image_data, [width as usize, height as usize]));
        } else {
            log::error!("Failed to load image from path: {}", path);
        }
    }

    /// Function to create the texture. this is separate from the load image
    /// function. This allows loading the image without having the context. The
    /// texture can be created later when egui updates by calling this function.
    ///
    /// Returns `true` if the texture was created and `false` if already created
    pub fn create_texture(&mut self, ctx: &egui::Context) -> bool {
        if self.texture.is_some() {
            return false;
        }

        if let Some((image_data, size)) = &self.img_data {
            let texture = ctx.load_texture(
                "image_texture",
                ColorImage::from_rgba_unmultiplied(*size, image_data),
                Default::default(),
            );

            self.texture = Some(texture);
            return true;
        }

        return false;
    }

    pub fn texture(&self) -> &Option<TextureHandle> {
        &self.texture
    }
}
