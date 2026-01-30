use crate::tree::{Container, ContainerType, LayoutMode, NodeId, Rectangle, WindowTree};
use tracing::{debug, info};

/// Maximum number of workspaces
pub const MAX_WORKSPACES: usize = 10;

/// Workspace manager handles multiple virtual desktops
pub struct WorkspaceManager {
    /// IDs of all workspaces (indexed 0-9 for workspaces 1-10)
    workspaces: [Option<NodeId>; MAX_WORKSPACES],

    /// Currently active workspace index (0-9)
    active_workspace: usize,

    /// Output node ID this workspace manager is attached to
    output_id: NodeId,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new(tree: &mut WindowTree, output_id: NodeId) -> Self {
        info!("Creating workspace manager for output {:?}", output_id);

        let mut workspaces = [None; MAX_WORKSPACES];

        // Create all 10 workspaces
        for i in 0..MAX_WORKSPACES {
            let mut workspace = Container::new(NodeId::default(), ContainerType::Workspace);
            workspace.layout = LayoutMode::SplitH; // Default to horizontal splits
            let workspace_id = tree.insert(workspace);

            // Add workspace as child of output
            if let Err(e) = tree.add_child(output_id, workspace_id) {
                tracing::error!("Failed to add workspace {} to output: {}", i + 1, e);
            }

            workspaces[i] = Some(workspace_id);
            debug!("Created workspace {} with id {:?}", i + 1, workspace_id);
        }

        Self {
            workspaces,
            active_workspace: 0,
            output_id,
        }
    }

    /// Get the currently active workspace ID
    pub fn active_workspace(&self) -> Option<NodeId> {
        self.workspaces[self.active_workspace]
    }

    /// Get the active workspace number (1-10)
    pub fn active_workspace_num(&self) -> usize {
        self.active_workspace + 1
    }

    /// Switch to a workspace by number (1-10)
    pub fn switch_to_workspace(&mut self, workspace_num: usize) -> Option<NodeId> {
        if workspace_num < 1 || workspace_num > MAX_WORKSPACES {
            tracing::warn!("Invalid workspace number: {}", workspace_num);
            return None;
        }

        let index = workspace_num - 1;
        info!("Switching to workspace {}", workspace_num);

        self.active_workspace = index;
        self.workspaces[index]
    }

    /// Get workspace ID by number (1-10)
    pub fn get_workspace(&self, workspace_num: usize) -> Option<NodeId> {
        if workspace_num < 1 || workspace_num > MAX_WORKSPACES {
            return None;
        }
        self.workspaces[workspace_num - 1]
    }

    /// Move a window to a different workspace
    pub fn move_window_to_workspace(
        &mut self,
        tree: &mut WindowTree,
        window_id: NodeId,
        target_workspace_num: usize,
    ) -> Result<(), String> {
        if target_workspace_num < 1 || target_workspace_num > MAX_WORKSPACES {
            return Err(format!("Invalid workspace number: {}", target_workspace_num));
        }

        let target_workspace_id = self.workspaces[target_workspace_num - 1]
            .ok_or("Target workspace not found")?;

        info!(
            "Moving window {:?} to workspace {}",
            window_id, target_workspace_num
        );

        // Get current parent
        let current_parent = tree
            .parent(window_id)
            .ok_or("Window has no parent")?;

        // Remove from current parent
        tree.remove_child(current_parent, window_id);

        // Add to target workspace
        tree.add_child(target_workspace_id, window_id)?;

        Ok(())
    }

    /// Get all workspace IDs
    pub fn all_workspaces(&self) -> Vec<Option<NodeId>> {
        self.workspaces.to_vec()
    }

    /// Check if a workspace has any windows
    pub fn workspace_has_windows(&self, tree: &WindowTree, workspace_num: usize) -> bool {
        if let Some(workspace_id) = self.get_workspace(workspace_num) {
            !tree.children(workspace_id).is_empty()
        } else {
            false
        }
    }

    /// Get the output ID this workspace manager is attached to
    pub fn output_id(&self) -> NodeId {
        self.output_id
    }

    /// Calculate and apply layout for the active workspace
    pub fn layout_active_workspace(&mut self, tree: &mut WindowTree, screen_geometry: Rectangle) {
        if let Some(workspace_id) = self.active_workspace() {
            debug!(
                "Laying out workspace {} with geometry {:?}",
                self.active_workspace_num(),
                screen_geometry
            );

            // Import the trait to use calculate_layout
            use crate::tree::WindowTreeExt;
            tree.calculate_layout(workspace_id, screen_geometry);
        }
    }

    /// Get list of visible windows on active workspace
    pub fn visible_windows(&self, tree: &WindowTree) -> Vec<NodeId> {
        let workspace_id = match self.active_workspace() {
            Some(id) => id,
            None => return vec![],
        };

        self.collect_windows_recursive(tree, workspace_id)
    }

    /// Recursively collect all windows under a container
    fn collect_windows_recursive(&self, tree: &WindowTree, container_id: NodeId) -> Vec<NodeId> {
        let mut windows = Vec::new();

        if let Some(container) = tree.get(container_id) {
            if container.container_type == ContainerType::Window {
                windows.push(container_id);
            } else {
                for &child_id in &container.children {
                    windows.extend(self.collect_windows_recursive(tree, child_id));
                }
            }
        }

        windows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_manager() {
        let mut tree = WindowTree::new();

        // Create output
        let output = Container::new(NodeId::default(), ContainerType::Output);
        let output_id = tree.insert(output);

        // Create workspace manager
        let mut manager = WorkspaceManager::new(&mut tree, output_id);

        // Test initial state
        assert_eq!(manager.active_workspace_num(), 1);
        assert!(manager.active_workspace().is_some());

        // Test switching workspaces
        manager.switch_to_workspace(5);
        assert_eq!(manager.active_workspace_num(), 5);

        // Test invalid workspace
        assert!(manager.switch_to_workspace(11).is_none());
    }
}
