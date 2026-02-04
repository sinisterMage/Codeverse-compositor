pub mod decorations;
pub mod wallpaper;

pub use decorations::{create_border_elements, BorderRenderElement};
pub use wallpaper::{
    load_cached_wallpaper, load_wallpaper_image, make_wallpaper_key, scale_wallpaper,
    CachedWallpaper, ScaleMode, WallpaperCache, WallpaperCacheKey,
};

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::{ImportDmaWl, ImportMemWl, Renderer};

// Create a combined render element type for DRM output that can hold
// window surfaces and border elements (solid colors)
// Wallpaper textures are rendered separately before the main elements
smithay::backend::renderer::element::render_elements! {
    pub OutputRenderElements<R> where R: Renderer + ImportMemWl + ImportDmaWl;
    Surface=WaylandSurfaceRenderElement<R>,
    Solid=SolidColorRenderElement,
}
