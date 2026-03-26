// ASCII dependency graph renderer for master plan children.
//
// Algorithm: build adjacency from `depends_on` (stores parent id as string),
// then DFS from each root to produce chains. Parallel branches (multiple
// children of the same node) each get their own output line.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::tui::data::ProjectTreeNode;
use crate::tui::widgets::{ACCENT, FAIL, MUTED, OK, TEXT_SECONDARY, WARN};

// Arrow separator between nodes.
const ARROW: &str = " \u{2192} "; // " → "

fn node_color(status: &str) -> ratatui::style::Color {
    match status {
        "done" => OK,
        "doing" => WARN,
        "blocked" => FAIL,
        _ => MUTED, // todo, draft, cancelled
    }
}

fn node_icon(status: &str) -> &'static str {
    match status {
        "done" => "\u{2713}",     // ✓
        "doing" => "\u{25c9}",    // ◉
        "blocked" => "\u{2715}",  // ✕
        _ => "\u{25cb}",          // ○
    }
}

/// Returns all chains from `start_idx` depth-first.
/// Each path from root to leaf becomes one Vec of node indices.
/// `nodes` is not used directly — only `children_map` drives traversal.
fn collect_chains(
    start_idx: usize,
    children_map: &[Vec<usize>],
    prefix: &[usize],
) -> Vec<Vec<usize>> {
    let mut path = prefix.to_vec();
    path.push(start_idx);

    let kids = &children_map[start_idx];
    if kids.is_empty() {
        return vec![path];
    }

    let mut result = Vec::new();
    for &child_idx in kids {
        let sub = collect_chains(child_idx, children_map, &path);
        result.extend(sub);
    }
    result
}

/// Build a styled `Line` for a single chain of nodes joined by arrows.
fn render_chain(chain: &[usize], nodes: &[ProjectTreeNode]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::styled("  ", Style::default())];

    for (pos, &idx) in chain.iter().enumerate() {
        let n = &nodes[idx];
        let icon = node_icon(&n.status);
        let color = node_color(&n.status);

        // "✓ Plan H0"
        spans.push(Span::styled(
            format!("{} {}", icon, n.name.clone()),
            Style::default().fg(color),
        ));

        if pos + 1 < chain.len() {
            spans.push(Span::styled(ARROW, Style::default().fg(TEXT_SECONDARY)));
        }
    }

    Line::from(spans)
}

/// Build ASCII dependency graph lines from a flat list of plan children.
///
/// Returns `Vec<Line<'static>>` suitable for inclusion in a ratatui widget.
/// Empty input → empty output.
pub fn build_dep_graph(nodes: &[ProjectTreeNode]) -> Vec<Line<'static>> {
    if nodes.is_empty() {
        return vec![];
    }

    // Build id→index lookup (depends_on stores parent id as string).
    let id_to_idx: std::collections::HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.to_string(), i))
        .collect();

    // children_map[i] = list of node indices whose `depends_on` points to nodes[i].
    let mut children_map: Vec<Vec<usize>> = vec![vec![]; nodes.len()];
    let mut has_parent = vec![false; nodes.len()];

    for (i, n) in nodes.iter().enumerate() {
        if let Some(dep_id) = &n.depends_on {
            if let Some(&parent_idx) = id_to_idx.get(dep_id) {
                children_map[parent_idx].push(i);
                has_parent[i] = true;
            }
        }
    }

    // Roots: nodes with no resolved parent.
    let roots: Vec<usize> = (0..nodes.len()).filter(|&i| !has_parent[i]).collect();

    // Collect all chains starting from each root.
    let mut all_chains: Vec<Vec<usize>> = Vec::new();
    for root_idx in roots {
        let chains = collect_chains(root_idx, &children_map, &[]);
        all_chains.extend(chains);
    }

    // Deduplicate chains that share a common prefix with a longer chain.
    // (A chain [A,B] is redundant when [A,B,C] also exists — B is not a leaf.)
    // We keep only chains where the last node is a true leaf OR the chain is maximal.
    // Simple approach: remove any chain that is a strict prefix of another.
    let deduped: Vec<Vec<usize>> = all_chains
        .iter()
        .filter(|chain| {
            !all_chains
                .iter()
                .any(|other| other.len() > chain.len() && other.starts_with(chain))
        })
        .cloned()
        .collect();

    // Header line.
    let header = Line::from(Span::styled(
        "  Dependencies:",
        Style::default().fg(ACCENT),
    ));

    let mut lines = vec![header];
    for chain in &deduped {
        lines.push(render_chain(chain, nodes));
    }

    lines
}
