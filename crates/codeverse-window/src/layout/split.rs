use crate::tree::{Container, NodeId, Orientation, Rectangle, WindowTree};

/// Calculate split layout for containers
pub struct SplitLayout {
    /// Gap between windows (in pixels)
    pub gap_width: i32,

    /// Border width for windows (in pixels)
    pub border_width: u32,
}

impl SplitLayout {
    /// Create a new split layout calculator
    pub fn new() -> Self {
        Self {
            gap_width: 4,
            border_width: 2,
        }
    }

    /// Calculate layout for a horizontal split
    pub fn layout_horizontal(
        &self,
        tree: &mut WindowTree,
        parent_id: NodeId,
        geometry: Rectangle,
    ) {
        let children = tree.children(parent_id);
        if children.is_empty() {
            return;
        }

        let num_children = children.len();
        let total_gap = (num_children as i32 - 1) * self.gap_width;
        let available_width = geometry.width as i32 - total_gap;

        if available_width <= 0 {
            return;
        }

        let child_width = available_width / num_children as i32;
        let mut x = geometry.x;

        for (i, &child_id) in children.iter().enumerate() {
            // Calculate width for this child (last one gets remainder)
            let width = if i == num_children - 1 {
                (geometry.x + geometry.width as i32) - x
            } else {
                child_width
            };

            let child_geometry = Rectangle::new(
                x,
                geometry.y,
                width as u32,
                geometry.height,
            );

            // Set geometry for this child
            if let Some(child) = tree.get_mut(child_id) {
                child.geometry = child_geometry;
            }

            // Recursively layout this child if it has children
            self.layout_container(tree, child_id, child_geometry);

            x += width + self.gap_width;
        }
    }

    /// Calculate layout for a vertical split
    pub fn layout_vertical(
        &self,
        tree: &mut WindowTree,
        parent_id: NodeId,
        geometry: Rectangle,
    ) {
        let children = tree.children(parent_id);
        if children.is_empty() {
            return;
        }

        let num_children = children.len();
        let total_gap = (num_children as i32 - 1) * self.gap_width;
        let available_height = geometry.height as i32 - total_gap;

        if available_height <= 0 {
            return;
        }

        let child_height = available_height / num_children as i32;
        let mut y = geometry.y;

        for (i, &child_id) in children.iter().enumerate() {
            // Calculate height for this child (last one gets remainder)
            let height = if i == num_children - 1 {
                (geometry.y + geometry.height as i32) - y
            } else {
                child_height
            };

            let child_geometry = Rectangle::new(
                geometry.x,
                y,
                geometry.width,
                height as u32,
            );

            // Set geometry for this child
            if let Some(child) = tree.get_mut(child_id) {
                child.geometry = child_geometry;
            }

            // Recursively layout this child if it has children
            self.layout_container(tree, child_id, child_geometry);

            y += height + self.gap_width;
        }
    }

    /// Layout a container based on its layout mode
    fn layout_container(
        &self,
        tree: &mut WindowTree,
        container_id: NodeId,
        geometry: Rectangle,
    ) {
        let layout_orientation = tree
            .get(container_id)
            .and_then(|c| c.layout.orientation());

        match layout_orientation {
            Some(Orientation::Horizontal) => {
                self.layout_horizontal(tree, container_id, geometry);
            }
            Some(Orientation::Vertical) => {
                self.layout_vertical(tree, container_id, geometry);
            }
            None => {
                // No specific orientation (e.g., Stacking/Tabbed)
                // These will be handled in Phase 3
            }
        }
    }
}

impl Default for SplitLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{ContainerType, LayoutMode};

    #[test]
    fn test_horizontal_split() {
        let mut tree = WindowTree::new();
        let layout = SplitLayout::new();

        // Create parent container
        let mut parent = Container::new(NodeId::default(), ContainerType::Split);
        parent.layout = LayoutMode::SplitH;
        let parent_id = tree.insert(parent);

        // Create two child windows
        let child1 = Container::new(NodeId::default(), ContainerType::Window);
        let child1_id = tree.insert(child1);
        tree.add_child(parent_id, child1_id).unwrap();

        let child2 = Container::new(NodeId::default(), ContainerType::Window);
        let child2_id = tree.insert(child2);
        tree.add_child(parent_id, child2_id).unwrap();

        // Layout with 1000x1000 geometry
        let geometry = Rectangle::new(0, 0, 1000, 1000);
        layout.layout_horizontal(&mut tree, parent_id, geometry);

        // Check that children are laid out side by side
        let child1_geom = tree.get(child1_id).unwrap().geometry;
        let child2_geom = tree.get(child2_id).unwrap().geometry;

        assert_eq!(child1_geom.y, 0);
        assert_eq!(child2_geom.y, 0);
        assert!(child1_geom.x < child2_geom.x);
    }

    #[test]
    fn test_vertical_split() {
        let mut tree = WindowTree::new();
        let layout = SplitLayout::new();

        // Create parent container
        let mut parent = Container::new(NodeId::default(), ContainerType::Split);
        parent.layout = LayoutMode::SplitV;
        let parent_id = tree.insert(parent);

        // Create two child windows
        let child1 = Container::new(NodeId::default(), ContainerType::Window);
        let child1_id = tree.insert(child1);
        tree.add_child(parent_id, child1_id).unwrap();

        let child2 = Container::new(NodeId::default(), ContainerType::Window);
        let child2_id = tree.insert(child2);
        tree.add_child(parent_id, child2_id).unwrap();

        // Layout with 1000x1000 geometry
        let geometry = Rectangle::new(0, 0, 1000, 1000);
        layout.layout_vertical(&mut tree, parent_id, geometry);

        // Check that children are laid out top and bottom
        let child1_geom = tree.get(child1_id).unwrap().geometry;
        let child2_geom = tree.get(child2_id).unwrap().geometry;

        assert_eq!(child1_geom.x, 0);
        assert_eq!(child2_geom.x, 0);
        assert!(child1_geom.y < child2_geom.y);
    }
}
