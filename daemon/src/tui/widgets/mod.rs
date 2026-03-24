use ratatui::style::{Color, Style};

// --- Maranello color palette ---

pub const ACCENT_U32: u32 = 0x00FFC72C;
pub const BG_DARK_U32: u32 = 0x001A1A1A;
pub const OK_U32: u32 = 0x0022C55E;
pub const FAIL_U32: u32 = 0x00EF4444;
pub const WARN_U32: u32 = 0x00F59E0B;
pub const MUTED_U32: u32 = 0x006B7280;

// New surface/text palette tokens
pub const BG_SURFACE_U32: u32 = 0x00262626;
pub const TEXT_PRIMARY_U32: u32 = 0x00F3F4F6;
pub const TEXT_SECONDARY_U32: u32 = 0x009CA3AF;

pub const ACCENT: Color = Color::from_u32(ACCENT_U32);
pub const OK: Color = Color::from_u32(OK_U32);
pub const FAIL: Color = Color::from_u32(FAIL_U32);
pub const WARN: Color = Color::from_u32(WARN_U32);
pub const MUTED: Color = Color::from_u32(MUTED_U32);
pub const BG_SURFACE: Color = Color::from_u32(BG_SURFACE_U32);
pub const TEXT_PRIMARY: Color = Color::from_u32(TEXT_PRIMARY_U32);
pub const TEXT_SECONDARY: Color = Color::from_u32(TEXT_SECONDARY_U32);

pub fn selected_style() -> Style {
    Style::default().reversed()
}

pub mod agents;
pub mod kanban;
pub mod kpi;
pub mod shared;

pub use agents::agent_org_chart;
pub use kanban::plan_kanban;
pub use kpi::kpi_strip;
pub use shared::{mesh_status, progress_bar, progress_bar_line, spark, task_pipeline};
