//! DSX TUI — rendering and drawing layout helpers.

use crate::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

impl App {
    pub fn draw(&self, frame: &mut Frame) {
        let input_height = if self.pending_approval.is_some() {
            6
        } else {
            3
        };

        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(input_height),
                Constraint::Length(1),
            ])
            .split(frame.area());

        let top = main[0];

        let show_reasoning = !self.current_reasoning.is_empty()
            || (self.model == "v4-pro"
                && matches!(self.agent_task, crate::types::AgentTask::Running(_)));

        let mut horizontal_constraints = Vec::new();
        if self.show_file_tree && !self.file_tree.is_empty() {
            horizontal_constraints.push(Constraint::Percentage(20));
        }
        horizontal_constraints.push(Constraint::Min(40));
        if show_reasoning {
            horizontal_constraints.push(Constraint::Percentage(33));
        }

        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(horizontal_constraints)
            .split(top);

        let mut pane_idx = 0;

        if self.show_file_tree && !self.file_tree.is_empty() {
            self.draw_file_tree(frame, panes[pane_idx]);
            pane_idx += 1;
        }

        let main_workspace_area = panes[pane_idx];
        pane_idx += 1;

        if show_reasoning {
            self.draw_reasoning(frame, panes[pane_idx]);
        }

        if self.show_diff {
            self.draw_diff(frame, main_workspace_area);
        } else if self.show_settings {
            self.draw_settings(frame, main_workspace_area);
        } else if self.show_tools {
            self.draw_tools(frame, main_workspace_area);
        } else if self.show_context {
            self.draw_context(frame, main_workspace_area);
        } else {
            self.draw_workspace(frame, main_workspace_area);
        }

        self.draw_input(frame, main[1]);
        self.draw_status(frame, main[2]);
    }
}
