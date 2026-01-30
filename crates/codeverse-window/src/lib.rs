pub mod floating;
pub mod layout;
pub mod tree;
pub mod workspace;

// Re-export commonly used types
pub use floating::{FloatingManager, MouseOperation, ResizeEdge};
pub use layout::SplitLayout;
pub use tree::{
    Container, ContainerType, Direction, LayoutMode, NodeId, Orientation, Rectangle, WindowHandle,
    WindowTree, WindowTreeExt,
};
pub use workspace::{WorkspaceManager, MAX_WORKSPACES};
