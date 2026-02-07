use codeverse_config::NordColor;
use slotmap::{new_key_type, SlotMap};
use smithay::wayland::shell::xdg::ToplevelSurface;

/// Handle to a Wayland window surface
pub type WindowHandle = ToplevelSurface;

new_key_type! {
    /// Unique identifier for a container node
    pub struct NodeId;
}

/// Type of container in the window tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerType {
    /// Root of the entire tree
    Root,
    /// Physical display output
    Output,
    /// Virtual workspace (desktop)
    Workspace,
    /// Horizontal or vertical split container
    Split,
    /// Stacking layout container
    Stacked,
    /// Tabbed layout container
    Tabbed,
    /// Actual window with Wayland surface
    Window,
    /// Floating window container
    Floating,
}

/// Orientation for split containers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Layout mode for containers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Horizontal split (side by side)
    SplitH,
    /// Vertical split (top and bottom)
    SplitV,
    /// Stacking (like tabs, but show all title bars)
    Stacking,
    /// Tabbed (only show active window, tabs at top)
    Tabbed,
}

impl LayoutMode {
    pub fn orientation(&self) -> Option<Orientation> {
        match self {
            LayoutMode::SplitH => Some(Orientation::Horizontal),
            LayoutMode::SplitV => Some(Orientation::Vertical),
            _ => None,
        }
    }
}

/// Rectangle for geometry calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }
}

/// A node in the container tree
pub struct Container {
    /// Unique identifier
    pub id: NodeId,

    /// Type of this container
    pub container_type: ContainerType,

    /// Parent container (None for root)
    pub parent: Option<NodeId>,

    /// Child containers
    pub children: Vec<NodeId>,

    /// Geometry (position and size)
    pub geometry: Rectangle,

    /// Layout mode for this container's children
    pub layout: LayoutMode,

    /// Is this container focused?
    pub focused: bool,

    /// Border width (in pixels)
    pub border_width: u32,

    /// Current border color
    pub border_color: NordColor,

    /// Window data (only for ContainerType::Window)
    pub window: Option<WindowHandle>,

    /// Window title (for display)
    pub title: Option<String>,

    /// Application ID
    pub app_id: Option<String>,

    /// Is this a floating window?
    pub is_floating: bool,

    /// Original geometry before floating (for toggle back)
    pub floating_original_geometry: Option<Rectangle>,

    /// Last size sent to the client via send_configure (to avoid spamming)
    pub last_configured_size: Option<(u32, u32)>,
}

impl Container {
    /// Create a new container
    pub fn new(id: NodeId, container_type: ContainerType) -> Self {
        Self {
            id,
            container_type,
            parent: None,
            children: Vec::new(),
            geometry: Rectangle::new(0, 0, 0, 0),
            layout: LayoutMode::SplitH,
            focused: false,
            border_width: 2,
            border_color: NordColor::rgb(0x4c, 0x56, 0x6a), // nord3
            window: None,
            title: None,
            app_id: None,
            is_floating: false,
            floating_original_geometry: None,
            last_configured_size: None,
        }
    }

    /// Check if this container can have children
    pub fn can_have_children(&self) -> bool {
        !matches!(
            self.container_type,
            ContainerType::Window | ContainerType::Floating
        )
    }

    /// Check if this is a leaf container (no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Set the border color based on focus state and theme
    pub fn update_border_color(&mut self, focused_color: NordColor, unfocused_color: NordColor) {
        self.border_color = if self.focused {
            focused_color
        } else {
            unfocused_color
        };
    }
}

/// Window tree storage
pub struct WindowTree {
    /// All containers indexed by NodeId
    nodes: SlotMap<NodeId, Container>,

    /// Root node of the tree
    root: Option<NodeId>,

    /// Currently focused node
    focused: Option<NodeId>,
}

impl WindowTree {
    /// Create a new empty window tree
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            root: None,
            focused: None,
        }
    }

    /// Insert a new container and return its ID
    pub fn insert(&mut self, container: Container) -> NodeId {
        self.nodes.insert(container)
    }

    /// Get a container by ID
    pub fn get(&self, id: NodeId) -> Option<&Container> {
        self.nodes.get(id)
    }

    /// Get a mutable container by ID
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Container> {
        self.nodes.get_mut(id)
    }

    /// Remove a container by ID
    pub fn remove(&mut self, id: NodeId) -> Option<Container> {
        self.nodes.remove(id)
    }

    /// Set the root node
    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
    }

    /// Get the root node ID
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Set the focused node
    pub fn set_focused(&mut self, id: Option<NodeId>) {
        // Unfocus previous
        if let Some(old_focused) = self.focused {
            if let Some(container) = self.nodes.get_mut(old_focused) {
                container.focused = false;
            }
        }

        // Focus new
        self.focused = id;
        if let Some(new_focused) = id {
            if let Some(container) = self.nodes.get_mut(new_focused) {
                container.focused = true;
            }
        }
    }

    /// Get the currently focused node ID
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// Add a child to a container
    pub fn add_child(&mut self, parent_id: NodeId, child_id: NodeId) -> Result<(), String> {
        // Check if parent can have children
        if let Some(parent) = self.nodes.get(parent_id) {
            if !parent.can_have_children() {
                return Err(format!(
                    "Container {:?} cannot have children",
                    parent.container_type
                ));
            }
        } else {
            return Err("Parent container not found".to_string());
        }

        // Set parent relationship
        if let Some(child) = self.nodes.get_mut(child_id) {
            child.parent = Some(parent_id);
        }

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.push(child_id);
        }

        Ok(())
    }

    /// Remove a child from its parent
    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.retain(|&id| id != child_id);
        }

        if let Some(child) = self.nodes.get_mut(child_id) {
            child.parent = None;
        }
    }

    /// Find all windows (leaf nodes with type Window)
    pub fn find_windows(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, container)| container.container_type == ContainerType::Window)
            .map(|(id, _)| id)
            .collect()
    }

    /// Find a window by its ToplevelSurface
    pub fn find_window_by_handle(&self, window: &WindowHandle) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|(_, container)| {
                if let Some(ref handle) = container.window {
                    handle.wl_surface() == window.wl_surface()
                } else {
                    false
                }
            })
            .map(|(id, _)| id)
    }

    /// Iterate through all nodes in the tree
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Container)> {
        self.nodes.iter()
    }

    /// Get all children of a container
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.get(id)
            .map(|c| c.children.clone())
            .unwrap_or_default()
    }

    /// Get parent of a container
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.get(id).and_then(|c| c.parent)
    }
}

impl Default for WindowTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tree() {
        let mut tree = WindowTree::new();

        let root = Container::new(tree.insert(Container::new(NodeId::default(), ContainerType::Root)), ContainerType::Root);
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        assert_eq!(tree.root(), Some(root_id));
    }

    #[test]
    fn test_focus() {
        let mut tree = WindowTree::new();

        let container1 = Container::new(NodeId::default(), ContainerType::Window);
        let id1 = tree.insert(container1);

        tree.set_focused(Some(id1));
        assert_eq!(tree.focused(), Some(id1));
        assert!(tree.get(id1).unwrap().focused);
    }
}
