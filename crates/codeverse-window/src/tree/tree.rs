use super::container::{Container, ContainerType, LayoutMode, NodeId, Orientation, Rectangle, WindowHandle, WindowTree};
use tracing::{debug, warn};

/// Direction for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// Get the orientation that corresponds to this direction
    pub fn orientation(&self) -> Orientation {
        match self {
            Direction::Left | Direction::Right => Orientation::Horizontal,
            Direction::Up | Direction::Down => Orientation::Vertical,
        }
    }

    /// Check if moving in negative direction (left/up vs right/down)
    pub fn is_negative(&self) -> bool {
        matches!(self, Direction::Left | Direction::Up)
    }
}

/// Extended operations for WindowTree
pub trait WindowTreeExt {
    /// Insert a new window into the tree at the focused location
    fn insert_window(&mut self, window: WindowHandle, workspace_id: NodeId) -> Result<NodeId, String>;

    /// Remove a window from the tree
    fn remove_window(&mut self, window_id: NodeId) -> Result<(), String>;

    /// Navigate focus in a direction
    fn navigate_focus(&mut self, direction: Direction) -> Option<NodeId>;

    /// Split the currently focused container
    fn split_focused(&mut self, orientation: Orientation) -> Result<NodeId, String>;

    /// Calculate layout geometries for all visible containers
    fn calculate_layout(&mut self, workspace_id: NodeId, screen_geometry: Rectangle);

    /// Get the workspace containing a node
    fn find_workspace(&self, node_id: NodeId) -> Option<NodeId>;

    /// Get the first focusable descendant (usually a window)
    fn first_focusable_descendant(&self, node_id: NodeId) -> Option<NodeId>;

    /// Change the layout mode of the focused container's parent
    fn change_layout(&mut self, layout: LayoutMode) -> Result<(), String>;
}

impl WindowTreeExt for WindowTree {
    fn insert_window(&mut self, window: WindowHandle, workspace_id: NodeId) -> Result<NodeId, String> {
        debug!("Inserting new window into tree at workspace {:?}", workspace_id);

        // Create window container
        let window_id = {
            let mut container = Container::new(NodeId::default(), ContainerType::Window);
            container.window = Some(window);
            container.container_type = ContainerType::Window;
            self.insert(container)
        };

        // Get current focus to determine where to insert
        let insert_target = if let Some(focused_id) = self.focused() {
            // If we have a focused window, insert next to it
            if let Some(focused) = self.get(focused_id) {
                if focused.container_type == ContainerType::Window {
                    // Insert as sibling of focused window
                    focused.parent.unwrap_or(workspace_id)
                } else {
                    // Insert into focused container
                    focused_id
                }
            } else {
                workspace_id
            }
        } else {
            // No focus, insert directly into workspace
            workspace_id
        };

        // Check if target has children
        let children_count = self.children(insert_target).len();

        if children_count == 0 {
            // First window in container, add directly
            self.add_child(insert_target, window_id)?;
        } else {
            // Container already has children, need to create a split
            let target_layout = self.get(insert_target).map(|c| c.layout).unwrap_or(LayoutMode::SplitH);

            // Add as sibling
            self.add_child(insert_target, window_id)?;
        }

        // Focus the new window
        self.set_focused(Some(window_id));

        debug!("Window inserted with id {:?}", window_id);
        Ok(window_id)
    }

    fn remove_window(&mut self, window_id: NodeId) -> Result<(), String> {
        debug!("Removing window {:?} from tree", window_id);

        let parent_id = self.parent(window_id).ok_or("Window has no parent")?;

        // Remove from parent's children
        self.remove_child(parent_id, window_id);

        // If focused window is being removed, focus another
        if self.focused() == Some(window_id) {
            // Try to focus a sibling or parent
            let siblings = self.children(parent_id);
            if !siblings.is_empty() {
                self.set_focused(Some(siblings[0]));
            } else {
                self.set_focused(Some(parent_id));
            }
        }

        // Remove the window container
        self.remove(window_id);

        debug!("Window removed");
        Ok(())
    }

    fn navigate_focus(&mut self, direction: Direction) -> Option<NodeId> {
        let current = self.focused()?;

        debug!("Navigating focus {:?} from {:?}", direction, current);

        // Get parent to find siblings
        let parent_id = self.parent(current)?;
        let parent = self.get(parent_id)?;

        // Check if parent's layout matches navigation direction
        let parent_orientation = parent.layout.orientation()?;

        if parent_orientation != direction.orientation() {
            // Need to go up the tree to find matching orientation
            return self.navigate_focus_recursive(current, direction);
        }

        // Find current index among siblings
        let siblings = parent.children.clone();
        let current_index = siblings.iter().position(|&id| id == current)?;

        // Calculate target index
        let target_index = if direction.is_negative() {
            current_index.checked_sub(1)?
        } else {
            let next = current_index + 1;
            if next >= siblings.len() {
                return None;
            }
            next
        };

        let target_id = siblings[target_index];

        // If target is a container, find first focusable descendant
        let focus_target = self.first_focusable_descendant(target_id).unwrap_or(target_id);

        self.set_focused(Some(focus_target));
        debug!("Focus moved to {:?}", focus_target);

        Some(focus_target)
    }

    fn split_focused(&mut self, orientation: Orientation) -> Result<NodeId, String> {
        let focused_id = self.focused().ok_or("No focused container")?;

        debug!("Splitting focused container {:?} with {:?}", focused_id, orientation);

        // Get parent
        let parent_id = self.parent(focused_id).ok_or("Cannot split root")?;

        // Create new split container
        let split_id = {
            let mut split = Container::new(NodeId::default(), ContainerType::Split);
            split.layout = match orientation {
                Orientation::Horizontal => LayoutMode::SplitH,
                Orientation::Vertical => LayoutMode::SplitV,
            };
            self.insert(split)
        };

        // Remove focused from parent
        self.remove_child(parent_id, focused_id);

        // Add split to parent
        self.add_child(parent_id, split_id)?;

        // Add focused as child of split
        self.add_child(split_id, focused_id)?;

        debug!("Created split container {:?}", split_id);
        Ok(split_id)
    }

    fn calculate_layout(&mut self, workspace_id: NodeId, screen_geometry: Rectangle) {
        debug!("Calculating layout for workspace {:?}", workspace_id);

        // Set workspace geometry
        if let Some(workspace) = self.get_mut(workspace_id) {
            workspace.geometry = screen_geometry;
        }

        // Recursively layout children
        self.layout_container(workspace_id, screen_geometry);
    }

    fn find_workspace(&self, mut node_id: NodeId) -> Option<NodeId> {
        loop {
            let node = self.get(node_id)?;
            if node.container_type == ContainerType::Workspace {
                return Some(node_id);
            }
            node_id = node.parent?;
        }
    }

    fn first_focusable_descendant(&self, node_id: NodeId) -> Option<NodeId> {
        let node = self.get(node_id)?;

        // If it's a window, return it
        if node.container_type == ContainerType::Window {
            return Some(node_id);
        }

        // Otherwise, check first child
        if let Some(&first_child) = node.children.first() {
            self.first_focusable_descendant(first_child)
        } else {
            None
        }
    }

    fn change_layout(&mut self, layout: LayoutMode) -> Result<(), String> {
        let focused_id = self.focused().ok_or("No focused container")?;

        // Get parent of focused container (we change parent's layout, not the focused itself)
        let parent_id = self.parent(focused_id).ok_or("Focused container has no parent")?;

        // Check if parent can have a layout
        let parent = self.get(parent_id).ok_or("Parent not found")?;
        if !parent.can_have_children() {
            return Err("Parent cannot have children".to_string());
        }

        // Change the layout
        if let Some(parent) = self.get_mut(parent_id) {
            debug!("Changing layout of {:?} from {:?} to {:?}", parent_id, parent.layout, layout);
            parent.layout = layout;
        }

        Ok(())
    }
}

impl WindowTree {
    /// Navigate focus recursively up the tree
    fn navigate_focus_recursive(&mut self, current: NodeId, direction: Direction) -> Option<NodeId> {
        let parent_id = self.parent(current)?;
        let parent = self.get(parent_id)?;

        // Check if we can navigate in this parent
        if let Some(parent_orientation) = parent.layout.orientation() {
            if parent_orientation == direction.orientation() {
                // Try to navigate among siblings
                let siblings = parent.children.clone();
                let current_index = siblings.iter().position(|&id| id == current)?;

                let target_index = if direction.is_negative() {
                    current_index.checked_sub(1)?
                } else {
                    current_index + 1
                };

                if target_index < siblings.len() {
                    return Some(siblings[target_index]);
                }
            }
        }

        // Go up one more level
        self.navigate_focus_recursive(parent_id, direction)
    }

    /// Layout a container and its children
    fn layout_container(&mut self, container_id: NodeId, geometry: Rectangle) {
        let (layout, children) = {
            let container = match self.get(container_id) {
                Some(c) => c,
                None => return,
            };
            (container.layout, container.children.clone())
        };

        if children.is_empty() {
            return;
        }

        let num_children = children.len();
        let border_width = 2; // TODO: Get from theme
        let gap_width = 4; // TODO: Get from theme

        match layout {
            LayoutMode::SplitH => {
                // Horizontal split: divide width equally
                let child_width = (geometry.width as i32 - (num_children as i32 - 1) * gap_width) / num_children as i32;
                let mut x = geometry.x;

                for (i, &child_id) in children.iter().enumerate() {
                    let child_geometry = Rectangle::new(
                        x,
                        geometry.y,
                        child_width as u32,
                        geometry.height,
                    );

                    if let Some(child) = self.get_mut(child_id) {
                        child.geometry = child_geometry;
                    }

                    // Recursively layout this child
                    self.layout_container(child_id, child_geometry);

                    x += child_width + gap_width;
                }
            }
            LayoutMode::SplitV => {
                // Vertical split: divide height equally
                let child_height = (geometry.height as i32 - (num_children as i32 - 1) * gap_width) / num_children as i32;
                let mut y = geometry.y;

                for (i, &child_id) in children.iter().enumerate() {
                    let child_geometry = Rectangle::new(
                        geometry.x,
                        y,
                        geometry.width,
                        child_height as u32,
                    );

                    if let Some(child) = self.get_mut(child_id) {
                        child.geometry = child_geometry;
                    }

                    // Recursively layout this child
                    self.layout_container(child_id, child_geometry);

                    y += child_height + gap_width;
                }
            }
            LayoutMode::Stacking => {
                // Stacking layout: all windows fullscreen (Phase 3 simplified)
                for &child_id in &children {
                    if let Some(child) = self.get_mut(child_id) {
                        child.geometry = geometry;
                    }
                    self.layout_container(child_id, geometry);
                }
            }
            LayoutMode::Tabbed => {
                // Tabbed layout: all windows fullscreen with tab bar space reserved
                let tab_bar_height = 30;
                let content_geometry = Rectangle::new(
                    geometry.x,
                    geometry.y + tab_bar_height,
                    geometry.width,
                    geometry.height.saturating_sub(tab_bar_height as u32),
                );

                for &child_id in &children {
                    if let Some(child) = self.get_mut(child_id) {
                        child.geometry = content_geometry;
                    }
                    self.layout_container(child_id, content_geometry);
                }
            }
        }
    }
}
