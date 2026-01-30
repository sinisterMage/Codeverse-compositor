use crate::tree::{Container, ContainerType, NodeId, Rectangle, WindowTree};

/// Stacking layout - shows all windows in a stack with title bars visible
/// Similar to traditional stacking window managers, all windows are visible
/// but only the focused one is fully shown
pub struct StackingLayout {
    /// Height of title bar (in pixels)
    pub title_bar_height: u32,

    /// Gap between windows
    pub gap_width: i32,
}

impl StackingLayout {
    /// Create a new stacking layout calculator
    pub fn new() -> Self {
        Self {
            title_bar_height: 30,
            gap_width: 4,
        }
    }

    /// Calculate layout for stacking mode
    /// In stacking mode, windows are layered with title bars visible
    /// For Phase 3, we'll implement a simplified version where
    /// windows are shown fullscreen (full implementation in later phases)
    pub fn layout_stacking(
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
        // - In Phase 4+, we'll add proper stacking with visible title bars

        for &child_id in &children {
            // Set geometry for this child
            if let Some(child) = tree.get_mut(child_id) {
                child.geometry = geometry;
            }

            // Recursively layout this child if it has children
            self.layout_container(tree, child_id, geometry);
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
            self.layout_stacking(tree, container_id, geometry);
        }
    }
}

impl Default for StackingLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::LayoutMode;

    #[test]
    fn test_stacking_layout() {
        let mut tree = WindowTree::new();
        let layout = StackingLayout::new();

        // Create parent container
        let mut parent = Container::new(NodeId::default(), ContainerType::Split);
        parent.layout = LayoutMode::Stacking;
        let parent_id = tree.insert(parent);

        // Create three child windows
        for _ in 0..3 {
            let child = Container::new(NodeId::default(), ContainerType::Window);
            let child_id = tree.insert(child);
            tree.add_child(parent_id, child_id).unwrap();
        }

        // Layout with 1000x1000 geometry
        let geometry = Rectangle::new(0, 0, 1000, 1000);
        layout.layout_stacking(&mut tree, parent_id, geometry);

        // All children should have the same geometry (fullscreen)
        let children = tree.children(parent_id);
        for child_id in children {
            let child_geom = tree.get(child_id).unwrap().geometry;
            assert_eq!(child_geom.width, 1000);
            assert_eq!(child_geom.height, 1000);
        }
    }
}
