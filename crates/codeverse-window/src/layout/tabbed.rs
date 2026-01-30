use crate::tree::{Container, ContainerType, NodeId, Rectangle, WindowTree};

/// Tabbed layout - shows windows as tabs with only active window visible
/// Similar to browser tabs, only one window is shown at a time
pub struct TabbedLayout {
    /// Height of tab bar (in pixels)
    pub tab_bar_height: u32,

    /// Width of each tab
    pub tab_width: u32,

    /// Gap between tabs
    pub tab_gap: i32,
}

impl TabbedLayout {
    /// Create a new tabbed layout calculator
    pub fn new() -> Self {
        Self {
            tab_bar_height: 30,
            tab_width: 150,
            tab_gap: 2,
        }
    }

    /// Calculate layout for tabbed mode
    /// In tabbed mode, only the focused window is visible
    /// For Phase 3, we implement a simplified version where
    /// windows are shown fullscreen (full tab bar rendering in later phases)
    pub fn layout_tabbed(
        &self,
        tree: &mut WindowTree,
        parent_id: NodeId,
        geometry: Rectangle,
    ) {
        let children = tree.children(parent_id);
        if children.is_empty() {
            return;
        }

        // Get focused window
        let focused_id = tree.focused();

        // For Phase 3 simplified implementation:
        // - All windows get the same geometry (fullscreen within parent)
        // - Only the focused window will be rendered (handled by rendering logic)
        // - In Phase 4+, we'll add proper tab bar rendering

        // Reserve space for tab bar at the top
        let content_geometry = Rectangle::new(
            geometry.x,
            geometry.y + self.tab_bar_height as i32,
            geometry.width,
            geometry.height.saturating_sub(self.tab_bar_height),
        );

        for &child_id in &children {
            // Set geometry for this child
            if let Some(child) = tree.get_mut(child_id) {
                child.geometry = content_geometry;
            }

            // Recursively layout this child if it has children
            self.layout_container(tree, child_id, content_geometry);
        }
    }

    /// Layout a container based on its layout mode
    fn layout_container(
        &self,
        tree: &mut WindowTree,
        container_id: NodeId,
        geometry: Rectangle,
    ) {
        // For containers with children, recurse
        let children = tree.children(container_id);
        if !children.is_empty() {
            self.layout_tabbed(tree, container_id, geometry);
        }
    }
}

impl Default for TabbedLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::LayoutMode;

    #[test]
    fn test_tabbed_layout() {
        let mut tree = WindowTree::new();
        let layout = TabbedLayout::new();

        // Create parent container
        let mut parent = Container::new(NodeId::default(), ContainerType::Split);
        parent.layout = LayoutMode::Tabbed;
        let parent_id = tree.insert(parent);

        // Create three child windows
        for _ in 0..3 {
            let child = Container::new(NodeId::default(), ContainerType::Window);
            let child_id = tree.insert(child);
            tree.add_child(parent_id, child_id).unwrap();
        }

        // Layout with 1000x1000 geometry
        let geometry = Rectangle::new(0, 0, 1000, 1000);
        layout.layout_tabbed(&mut tree, parent_id, geometry);

        // All children should have the same geometry (minus tab bar)
        let children = tree.children(parent_id);
        for child_id in children {
            let child_geom = tree.get(child_id).unwrap().geometry;
            assert_eq!(child_geom.width, 1000);
            assert_eq!(child_geom.height, 970); // 1000 - 30 (tab bar height)
        }
    }
}
