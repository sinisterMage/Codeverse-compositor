use crate::tree::{NodeId, Rectangle, WindowTree};

/// State for mouse-based window operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseOperation {
    /// Moving a floating window
    Moving { window: NodeId, start_x: i32, start_y: i32, original_geometry: Rectangle },
    /// Resizing a floating window
    Resizing { window: NodeId, start_x: i32, start_y: i32, original_geometry: Rectangle, edge: ResizeEdge },
    /// No operation in progress
    None,
}

/// Edge or corner being used to resize
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Manages floating windows and their interactions
pub struct FloatingManager {
    /// Stacking order for floating windows (top-most last)
    /// Windows at the end of the vec are rendered on top
    stack: Vec<NodeId>,

    /// Current mouse operation (if any)
    operation: MouseOperation,

    /// Minimum window size
    min_width: u32,
    min_height: u32,

    /// Default floating window size
    default_width: u32,
    default_height: u32,

    /// Title bar height
    title_bar_height: u32,
}

impl FloatingManager {
    /// Create a new floating manager
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            operation: MouseOperation::None,
            min_width: 200,
            min_height: 100,
            default_width: 800,
            default_height: 600,
            title_bar_height: 30,
        }
    }

    /// Toggle a window between tiled and floating mode
    pub fn toggle_floating(
        &mut self,
        tree: &mut WindowTree,
        window_id: NodeId,
        screen_geometry: Rectangle,
    ) -> Result<(), String> {
        let container = tree.get(window_id)
            .ok_or("Window not found")?;

        if container.is_floating {
            // Make it tiled again
            self.make_tiled(tree, window_id)?;
        } else {
            // Make it floating
            self.make_floating(tree, window_id, screen_geometry)?;
        }

        Ok(())
    }

    /// Make a window floating
    fn make_floating(
        &mut self,
        tree: &mut WindowTree,
        window_id: NodeId,
        screen_geometry: Rectangle,
    ) -> Result<(), String> {
        let container = tree.get_mut(window_id)
            .ok_or("Window not found")?;

        // Store original geometry for toggle back
        container.floating_original_geometry = Some(container.geometry);
        container.is_floating = true;

        // Center the window with default size
        let x = screen_geometry.x + (screen_geometry.width as i32 - self.default_width as i32) / 2;
        let y = screen_geometry.y + (screen_geometry.height as i32 - self.default_height as i32) / 2;

        container.geometry = Rectangle::new(
            x,
            y,
            self.default_width,
            self.default_height,
        );

        // Add to stacking order (on top)
        if !self.stack.contains(&window_id) {
            self.stack.push(window_id);
        }

        Ok(())
    }

    /// Make a floating window tiled again
    fn make_tiled(
        &mut self,
        tree: &mut WindowTree,
        window_id: NodeId,
    ) -> Result<(), String> {
        let container = tree.get_mut(window_id)
            .ok_or("Window not found")?;

        container.is_floating = false;

        // Restore original geometry if available
        if let Some(original) = container.floating_original_geometry {
            container.geometry = original;
        }
        container.floating_original_geometry = None;

        // Remove from stacking order
        self.stack.retain(|&id| id != window_id);

        Ok(())
    }

    /// Start moving a floating window
    pub fn start_move(
        &mut self,
        tree: &WindowTree,
        window_id: NodeId,
        cursor_x: i32,
        cursor_y: i32,
    ) -> Result<(), String> {
        let container = tree.get(window_id)
            .ok_or("Window not found")?;

        if !container.is_floating {
            return Err("Window is not floating".to_string());
        }

        self.operation = MouseOperation::Moving {
            window: window_id,
            start_x: cursor_x,
            start_y: cursor_y,
            original_geometry: container.geometry,
        };

        // Bring window to front
        self.raise_window(window_id);

        Ok(())
    }

    /// Start resizing a floating window
    pub fn start_resize(
        &mut self,
        tree: &WindowTree,
        window_id: NodeId,
        cursor_x: i32,
        cursor_y: i32,
        edge: ResizeEdge,
    ) -> Result<(), String> {
        let container = tree.get(window_id)
            .ok_or("Window not found")?;

        if !container.is_floating {
            return Err("Window is not floating".to_string());
        }

        self.operation = MouseOperation::Resizing {
            window: window_id,
            start_x: cursor_x,
            start_y: cursor_y,
            original_geometry: container.geometry,
            edge,
        };

        // Bring window to front
        self.raise_window(window_id);

        Ok(())
    }

    /// Update ongoing mouse operation
    pub fn update_operation(
        &mut self,
        tree: &mut WindowTree,
        cursor_x: i32,
        cursor_y: i32,
    ) -> Result<(), String> {
        match self.operation {
            MouseOperation::Moving { window, start_x, start_y, original_geometry } => {
                let dx = cursor_x - start_x;
                let dy = cursor_y - start_y;

                if let Some(container) = tree.get_mut(window) {
                    container.geometry.x = original_geometry.x + dx;
                    container.geometry.y = original_geometry.y + dy;
                }
            }
            MouseOperation::Resizing { window, start_x, start_y, original_geometry, edge } => {
                let dx = cursor_x - start_x;
                let dy = cursor_y - start_y;

                if let Some(container) = tree.get_mut(window) {
                    let mut new_geometry = original_geometry;

                    match edge {
                        ResizeEdge::Right => {
                            new_geometry.width = (original_geometry.width as i32 + dx).max(self.min_width as i32) as u32;
                        }
                        ResizeEdge::Bottom => {
                            new_geometry.height = (original_geometry.height as i32 + dy).max(self.min_height as i32) as u32;
                        }
                        ResizeEdge::Left => {
                            let new_width = (original_geometry.width as i32 - dx).max(self.min_width as i32) as u32;
                            new_geometry.x = original_geometry.x + (original_geometry.width as i32 - new_width as i32);
                            new_geometry.width = new_width;
                        }
                        ResizeEdge::Top => {
                            let new_height = (original_geometry.height as i32 - dy).max(self.min_height as i32) as u32;
                            new_geometry.y = original_geometry.y + (original_geometry.height as i32 - new_height as i32);
                            new_geometry.height = new_height;
                        }
                        ResizeEdge::TopLeft => {
                            let new_width = (original_geometry.width as i32 - dx).max(self.min_width as i32) as u32;
                            let new_height = (original_geometry.height as i32 - dy).max(self.min_height as i32) as u32;
                            new_geometry.x = original_geometry.x + (original_geometry.width as i32 - new_width as i32);
                            new_geometry.y = original_geometry.y + (original_geometry.height as i32 - new_height as i32);
                            new_geometry.width = new_width;
                            new_geometry.height = new_height;
                        }
                        ResizeEdge::TopRight => {
                            let new_width = (original_geometry.width as i32 + dx).max(self.min_width as i32) as u32;
                            let new_height = (original_geometry.height as i32 - dy).max(self.min_height as i32) as u32;
                            new_geometry.y = original_geometry.y + (original_geometry.height as i32 - new_height as i32);
                            new_geometry.width = new_width;
                            new_geometry.height = new_height;
                        }
                        ResizeEdge::BottomLeft => {
                            let new_width = (original_geometry.width as i32 - dx).max(self.min_width as i32) as u32;
                            let new_height = (original_geometry.height as i32 + dy).max(self.min_height as i32) as u32;
                            new_geometry.x = original_geometry.x + (original_geometry.width as i32 - new_width as i32);
                            new_geometry.width = new_width;
                            new_geometry.height = new_height;
                        }
                        ResizeEdge::BottomRight => {
                            new_geometry.width = (original_geometry.width as i32 + dx).max(self.min_width as i32) as u32;
                            new_geometry.height = (original_geometry.height as i32 + dy).max(self.min_height as i32) as u32;
                        }
                    }

                    container.geometry = new_geometry;
                }
            }
            MouseOperation::None => {}
        }

        Ok(())
    }

    /// Finish the current mouse operation
    pub fn finish_operation(&mut self) {
        self.operation = MouseOperation::None;
    }

    /// Get the current operation
    pub fn current_operation(&self) -> MouseOperation {
        self.operation
    }

    /// Raise a window to the top of the stacking order
    pub fn raise_window(&mut self, window_id: NodeId) {
        // Remove from current position
        self.stack.retain(|&id| id != window_id);
        // Add to top
        self.stack.push(window_id);
    }

    /// Get floating windows in stacking order (bottom to top)
    pub fn get_stack(&self) -> &[NodeId] {
        &self.stack
    }

    /// Remove a window from the floating manager
    pub fn remove_window(&mut self, window_id: NodeId) {
        self.stack.retain(|&id| id != window_id);

        // Cancel operation if it involves this window
        match self.operation {
            MouseOperation::Moving { window, .. } | MouseOperation::Resizing { window, .. }
                if window == window_id => {
                self.operation = MouseOperation::None;
            }
            _ => {}
        }
    }

    /// Find the topmost floating window at a given position
    pub fn find_window_at(&self, tree: &WindowTree, x: i32, y: i32) -> Option<NodeId> {
        // Search from top to bottom (reverse order)
        for &window_id in self.stack.iter().rev() {
            if let Some(container) = tree.get(window_id) {
                if container.is_floating && container.geometry.contains_point(x, y) {
                    return Some(window_id);
                }
            }
        }
        None
    }

    /// Get title bar height
    pub fn title_bar_height(&self) -> u32 {
        self.title_bar_height
    }

    /// Check if a point is in the title bar of a floating window
    pub fn is_in_title_bar(&self, tree: &WindowTree, window_id: NodeId, x: i32, y: i32) -> bool {
        if let Some(container) = tree.get(window_id) {
            if !container.is_floating {
                return false;
            }

            let geom = container.geometry;
            x >= geom.x
                && x < geom.x + geom.width as i32
                && y >= geom.y
                && y < geom.y + self.title_bar_height as i32
        } else {
            false
        }
    }

    /// Detect which resize edge is closest to a point on the window border
    pub fn detect_resize_edge(&self, tree: &WindowTree, window_id: NodeId, x: i32, y: i32) -> Option<ResizeEdge> {
        let container = tree.get(window_id)?;
        if !container.is_floating {
            return None;
        }

        let geom = container.geometry;
        let border_threshold = 10; // pixels from edge

        let left = (x - geom.x).abs() < border_threshold;
        let right = (x - (geom.x + geom.width as i32)).abs() < border_threshold;
        let top = (y - geom.y).abs() < border_threshold;
        let bottom = (y - (geom.y + geom.height as i32)).abs() < border_threshold;

        // Check corners first
        if top && left {
            Some(ResizeEdge::TopLeft)
        } else if top && right {
            Some(ResizeEdge::TopRight)
        } else if bottom && left {
            Some(ResizeEdge::BottomLeft)
        } else if bottom && right {
            Some(ResizeEdge::BottomRight)
        } else if top {
            Some(ResizeEdge::Top)
        } else if bottom {
            Some(ResizeEdge::Bottom)
        } else if left {
            Some(ResizeEdge::Left)
        } else if right {
            Some(ResizeEdge::Right)
        } else {
            None
        }
    }
}

impl Default for FloatingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Container, ContainerType, WindowTree};

    #[test]
    fn test_toggle_floating() {
        let mut tree = WindowTree::new();
        let mut floating_mgr = FloatingManager::new();

        // Create a window
        let mut container = Container::new(NodeId::default(), ContainerType::Window);
        container.geometry = Rectangle::new(0, 0, 400, 300);
        let window_id = tree.insert(container);

        let screen = Rectangle::new(0, 0, 1920, 1080);

        // Make it floating
        floating_mgr.toggle_floating(&mut tree, window_id, screen).unwrap();
        assert!(tree.get(window_id).unwrap().is_floating);
        assert_eq!(floating_mgr.stack.len(), 1);

        // Make it tiled again
        floating_mgr.toggle_floating(&mut tree, window_id, screen).unwrap();
        assert!(!tree.get(window_id).unwrap().is_floating);
        assert_eq!(floating_mgr.stack.len(), 0);
    }

    #[test]
    fn test_move_window() {
        let mut tree = WindowTree::new();
        let mut floating_mgr = FloatingManager::new();

        // Create a floating window
        let mut container = Container::new(NodeId::default(), ContainerType::Window);
        container.geometry = Rectangle::new(100, 100, 400, 300);
        container.is_floating = true;
        let window_id = tree.insert(container);
        floating_mgr.stack.push(window_id);

        // Start move
        floating_mgr.start_move(&tree, window_id, 150, 150).unwrap();

        // Update position
        floating_mgr.update_operation(&mut tree, 200, 200).unwrap();

        // Check new position (moved by 50, 50)
        let geom = tree.get(window_id).unwrap().geometry;
        assert_eq!(geom.x, 150);
        assert_eq!(geom.y, 150);

        floating_mgr.finish_operation();
    }

    #[test]
    fn test_resize_window() {
        let mut tree = WindowTree::new();
        let mut floating_mgr = FloatingManager::new();

        // Create a floating window
        let mut container = Container::new(NodeId::default(), ContainerType::Window);
        container.geometry = Rectangle::new(100, 100, 400, 300);
        container.is_floating = true;
        let window_id = tree.insert(container);
        floating_mgr.stack.push(window_id);

        // Start resize from bottom-right
        floating_mgr.start_resize(&tree, window_id, 500, 400, ResizeEdge::BottomRight).unwrap();

        // Resize by dragging +50, +50
        floating_mgr.update_operation(&mut tree, 550, 450).unwrap();

        // Check new size
        let geom = tree.get(window_id).unwrap().geometry;
        assert_eq!(geom.width, 450);
        assert_eq!(geom.height, 350);

        floating_mgr.finish_operation();
    }

    #[test]
    fn test_stacking_order() {
        let mut tree = WindowTree::new();
        let mut floating_mgr = FloatingManager::new();

        // Create three distinct windows
        let mut container1 = Container::new(NodeId::default(), ContainerType::Window);
        container1.is_floating = true;
        let id1 = tree.insert(container1);

        let mut container2 = Container::new(NodeId::default(), ContainerType::Window);
        container2.is_floating = true;
        let id2 = tree.insert(container2);

        let mut container3 = Container::new(NodeId::default(), ContainerType::Window);
        container3.is_floating = true;
        let id3 = tree.insert(container3);

        floating_mgr.stack.push(id1);
        floating_mgr.stack.push(id2);
        floating_mgr.stack.push(id3);

        // Raise id1 to top
        floating_mgr.raise_window(id1);

        assert_eq!(floating_mgr.stack, vec![id2, id3, id1]);
    }
}
