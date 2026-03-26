// Dependency graph renderer tests — TDD RED phase.
// Tests written before implementation per TDD mandate.
use crate::tui::data::ProjectTreeNode;
use crate::tui::views::dep_graph::build_dep_graph;
use crate::tui::widgets::{FAIL, MUTED, OK, WARN};

fn node(id: i64, name: &str, status: &str, depends_on: Option<&str>) -> ProjectTreeNode {
    ProjectTreeNode {
        id,
        name: name.to_string(),
        status: status.to_string(),
        depends_on: depends_on.map(|s| s.to_string()),
        ..Default::default()
    }
}

fn lines_to_text(lines: &[ratatui::text::Line<'_>]) -> String {
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// --- Linear chain: A → B → C ---

#[test]
fn linear_chain_renders_arrow_sequence() {
    let nodes = vec![
        node(719, "Plan H0", "done", None),
        node(712, "Plan H", "doing", Some("719")),
        node(713, "Plan I", "todo", Some("712")),
    ];
    let lines = build_dep_graph(&nodes);
    let text = lines_to_text(&lines);
    assert!(text.contains("Plan H0"), "missing Plan H0: {text}");
    assert!(text.contains("Plan H"), "missing Plan H: {text}");
    assert!(text.contains("Plan I"), "missing Plan I: {text}");
    // Arrow connector must appear
    assert!(text.contains('\u{2192}'), "missing arrow (→): {text}");
}

#[test]
fn linear_chain_all_on_one_branch_line() {
    // A → B → C with no branching should appear together (same line or sequential chain).
    let nodes = vec![
        node(1, "Plan H0", "done", None),
        node(2, "Plan H", "doing", Some("1")),
        node(3, "Plan I", "todo", Some("2")),
    ];
    let lines = build_dep_graph(&nodes);
    let text = lines_to_text(&lines);
    // At least one line contains all three in order
    let chain_line = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .find(|s| s.contains("Plan H0") && s.contains("Plan H") && s.contains("Plan I"));
    assert!(
        chain_line.is_some(),
        "expected a single line with full chain A→B→C: {text}"
    );
}

// --- Parallel branches: A → B, A → C ---

#[test]
fn parallel_branches_render_on_separate_lines() {
    let nodes = vec![
        node(719, "Plan H0", "done", None),
        node(712, "Plan H", "doing", Some("719")),
        node(713, "Plan I", "todo", Some("719")),
    ];
    let lines = build_dep_graph(&nodes);
    let text = lines_to_text(&lines);
    assert!(text.contains("Plan H"), "missing Plan H: {text}");
    assert!(text.contains("Plan I"), "missing Plan I: {text}");
    assert!(text.contains("Plan H0"), "missing Plan H0: {text}");
}

#[test]
fn parallel_branches_produce_multiple_arrow_rows() {
    let nodes = vec![
        node(719, "Plan H0", "done", None),
        node(712, "Plan H", "doing", Some("719")),
        node(713, "Plan I", "todo", Some("719")),
    ];
    let lines = build_dep_graph(&nodes);
    // Each branch of a fork should be on its own line → at least 2 lines with arrows
    let arrow_lines: Vec<_> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .filter(|s| s.contains('\u{2192}'))
        .collect();
    assert!(
        arrow_lines.len() >= 2,
        "expected >=2 arrow-lines for parallel: {arrow_lines:?}"
    );
}

// --- No dependencies: standalone nodes listed ---

#[test]
fn no_deps_all_nodes_listed() {
    let nodes = vec![
        node(1, "Plan H0", "done", None),
        node(2, "Plan H", "doing", None),
        node(3, "Plan I", "todo", None),
    ];
    let lines = build_dep_graph(&nodes);
    let text = lines_to_text(&lines);
    assert!(text.contains("Plan H0"), "missing Plan H0: {text}");
    assert!(text.contains("Plan H"), "missing Plan H: {text}");
    assert!(text.contains("Plan I"), "missing Plan I: {text}");
}

// --- Status colors reflected ---

#[test]
fn done_status_uses_ok_color() {
    let nodes = vec![node(1, "Plan H0", "done", None)];
    let lines = build_dep_graph(&nodes);
    let has_ok = lines
        .iter()
        .any(|l| l.spans.iter().any(|s| s.style.fg == Some(OK)));
    assert!(has_ok, "done node should use OK color");
}

#[test]
fn doing_status_uses_warn_color() {
    let nodes = vec![node(1, "Plan H", "doing", None)];
    let lines = build_dep_graph(&nodes);
    let has_warn = lines
        .iter()
        .any(|l| l.spans.iter().any(|s| s.style.fg == Some(WARN)));
    assert!(has_warn, "doing node should use WARN color");
}

#[test]
fn blocked_status_uses_fail_color() {
    let nodes = vec![node(1, "Plan H", "blocked", None)];
    let lines = build_dep_graph(&nodes);
    let has_fail = lines
        .iter()
        .any(|l| l.spans.iter().any(|s| s.style.fg == Some(FAIL)));
    assert!(has_fail, "blocked node should use FAIL color");
}

#[test]
fn todo_status_uses_muted_color() {
    let nodes = vec![node(1, "Plan I", "todo", None)];
    let lines = build_dep_graph(&nodes);
    let has_muted = lines
        .iter()
        .any(|l| l.spans.iter().any(|s| s.style.fg == Some(MUTED)));
    assert!(has_muted, "todo node should use MUTED color");
}

// --- Empty children → empty graph ---

#[test]
fn empty_nodes_returns_empty_lines() {
    let nodes: Vec<ProjectTreeNode> = vec![];
    let lines = build_dep_graph(&nodes);
    let content_lines: Vec<_> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .filter(|s| !s.trim().is_empty())
        .collect();
    assert!(
        content_lines.is_empty(),
        "empty input should produce no content lines: {content_lines:?}"
    );
}

// --- Convergio Vision master plan integration ---

#[test]
fn convergio_vision_master_renders_correctly() {
    // Mirrors real convergio-vision master spec dependency_graph:
    // H0 → H → I → J → L
    // H → M → N
    // I → K (parallel with J)
    let nodes = vec![
        node(719, "Plan H0", "done", None),
        node(712, "Plan H", "doing", Some("719")),
        node(713, "Plan I", "todo", Some("712")),
        node(714, "Plan J", "todo", Some("713")),
        node(715, "Plan K", "todo", Some("713")),
        node(716, "Plan L", "todo", Some("714")),
        node(717, "Plan M", "todo", Some("712")),
        node(718, "Plan N", "todo", Some("717")),
    ];
    let lines = build_dep_graph(&nodes);
    let text = lines_to_text(&lines);

    for name in &["Plan H0", "Plan H", "Plan I", "Plan J", "Plan K", "Plan L", "Plan M", "Plan N"]
    {
        assert!(text.contains(name), "missing {name}: {text}");
    }
    assert!(text.contains('\u{2192}'), "missing → arrows");

    // Multiple parallel branches → multiple arrow rows
    let arrow_rows: Vec<_> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .filter(|s| s.contains('\u{2192}'))
        .collect();
    assert!(
        arrow_rows.len() >= 3,
        "expected >=3 arrow-lines for complex graph: {arrow_rows:?}"
    );
}
