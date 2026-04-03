/// Obsidian vault exporter.
///
/// Writes one markdown file per PKG node. Files use YAML frontmatter so
/// Obsidian's Dataview plugin can query them:
///
///   TABLE rocky_retrievability, rocky_last_reviewed
///   FROM #rocky/node
///   WHERE rocky_retrievability < 0.7
///   SORT rocky_retrievability ASC
use std::path::Path;

use anyhow::Result;
use chrono::Local;

use crate::fsrs;
use crate::node::Node;

pub fn write_node(node: &Node, vault_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(vault_dir)?;

    let r = fsrs::retrievability(node.stability, node.last_reviewed);
    let today = Local::now().date_naive();
    let days_since = (today - node.last_reviewed).num_days();

    let frontmatter = format!(
        "---\nrocky_id: {}\nrocky_kind: {}\nrocky_difficulty: {}\nrocky_stability: {}\nrocky_retrievability: {}\nrocky_last_reviewed: {}\nrocky_last_encountered: {}\nrocky_review_count: {}\nrocky_days_since_review: {}\ntags: [rocky/node, rocky/kind/{}]\n---\n",
        node.id,
        node.kind.as_str(),
        node.difficulty,
        node.stability,
        r,
        node.last_reviewed,
        node.last_encountered,
        node.review_count,
        days_since,
        node.kind.as_str(),
    );

    let mut body = format!("# {}\n\n", node.topic);
    if !node.description.is_empty() {
        body.push_str(&node.description);
        body.push('\n');
    }
    if !node.contexts.is_empty() {
        body.push_str("\n## Contexts\n");
        for ctx in &node.contexts {
            body.push_str(&format!("- {ctx}\n"));
        }
    }

    let content = format!("{frontmatter}\n{body}");
    let file_path = vault_dir.join(format!("{}.md", node.id));
    std::fs::write(file_path, content)?;
    Ok(())
}

pub fn write_all(nodes: &[Node], vault_dir: &Path) -> Result<usize> {
    std::fs::create_dir_all(vault_dir)?;
    for node in nodes {
        write_node(node, vault_dir)?;
    }
    Ok(nodes.len())
}
