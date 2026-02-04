use codeverse_config::NordColor;
use smithay::{
    backend::renderer::{
        element::{
            solid::SolidColorRenderElement,
            Id, Kind,
        },
        Color32F,
    },
    utils::{Physical, Rectangle},
};

/// Render element for window borders
pub type BorderRenderElement = SolidColorRenderElement;

/// Create border render elements for a window
///
/// Creates four SolidColorRenderElements representing the top, bottom,
/// left, and right borders around a window.
pub fn create_border_elements(
    window_rect: Rectangle<i32, Physical>,
    border_width: u32,
    color: NordColor,
    id: &str,
) -> Vec<BorderRenderElement> {
    if border_width == 0 {
        return vec![];
    }

    let mut elements = Vec::with_capacity(4);
    let border_width = border_width as i32;
    let color_array = color.to_f32_array();
    let color = Color32F::new(color_array[0], color_array[1], color_array[2], color_array[3]);

    let x = window_rect.loc.x;
    let y = window_rect.loc.y;
    let width = window_rect.size.w;
    let height = window_rect.size.h;

    // Top border
    let top_rect = Rectangle::from_loc_and_size(
        (x - border_width, y - border_width),
        (width + border_width * 2, border_width),
    );
    elements.push(SolidColorRenderElement::new(
        Id::new(),
        top_rect,
        1,  // commit counter
        color,
        Kind::Unspecified,
    ));

    // Bottom border
    let bottom_rect = Rectangle::from_loc_and_size(
        (x - border_width, y + height),
        (width + border_width * 2, border_width),
    );
    elements.push(SolidColorRenderElement::new(
        Id::new(),
        bottom_rect,
        1,
        color,
        Kind::Unspecified,
    ));

    // Left border
    let left_rect = Rectangle::from_loc_and_size(
        (x - border_width, y),
        (border_width, height),
    );
    elements.push(SolidColorRenderElement::new(
        Id::new(),
        left_rect,
        1,
        color,
        Kind::Unspecified,
    ));

    // Right border
    let right_rect = Rectangle::from_loc_and_size(
        (x + width, y),
        (border_width, height),
    );
    elements.push(SolidColorRenderElement::new(
        Id::new(),
        right_rect,
        1,
        color,
        Kind::Unspecified,
    ));

    // Suppress unused id warning - id can be used for debugging later
    let _ = id;

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_borders() {
        let rect: Rectangle<i32, Physical> = Rectangle::from_loc_and_size((100, 100), (400, 300));
        let color = NordColor::rgb(0x88, 0xc0, 0xd0);
        let borders = create_border_elements(rect, 2, color, "test-window");

        // Should create 4 border elements
        assert_eq!(borders.len(), 4);
    }

    #[test]
    fn test_zero_border_width() {
        let rect: Rectangle<i32, Physical> = Rectangle::from_loc_and_size((100, 100), (400, 300));
        let color = NordColor::rgb(0x88, 0xc0, 0xd0);
        let borders = create_border_elements(rect, 0, color, "test-window");

        // Zero border width should return empty
        assert_eq!(borders.len(), 0);
    }
}
