//! Workflow panel rendering for compact plan state and tool timeline.

use crate::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

impl App {
    pub fn draw_workspace(&self, frame: &mut Frame, area: Rect) {
        if area.width < 82 || area.height < 14 {
            self.draw_chat(frame, area);
            return;
        }

        if area.width >= 116 {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(56), Constraint::Length(42)])
                .split(area);
            self.draw_chat(frame, panes[0]);
            self.draw_workflow_panel(frame, panes[1]);
        } else {
            let panes = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(8), Constraint::Length(10)])
                .split(area);
            self.draw_chat(frame, panes[0]);
            self.draw_workflow_panel(frame, panes[1]);
        }
    }

    fn draw_workflow_panel(&self, frame: &mut Frame, area: Rect) {
        if area.height >= 24 {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(34),
                    Constraint::Length(8),
                    Constraint::Length(7),
                    Constraint::Min(6),
                ])
                .split(area);
            self.draw_task_brief_panel(frame, split[0]);
            self.draw_scope_lock_panel(frame, split[1]);
            self.draw_folder_notes_panel(frame, split[2]);
            self.draw_tool_timeline_panel(frame, split[3]);
            return;
        }

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(44),
                Constraint::Length(8),
                Constraint::Min(6),
            ])
            .split(area);
        self.draw_task_brief_panel(frame, split[0]);
        self.draw_scope_lock_panel(frame, split[1]);
        self.draw_tool_timeline_panel(frame, split[2]);
    }
}
