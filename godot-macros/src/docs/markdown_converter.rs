/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Converts [Markdown](https://en.wikipedia.org/wiki/Markdown) to Godot-compatible [BBCode](https://en.wikipedia.org/wiki/BBCode).

use markdown::mdast as md;
use markdown::{to_mdast, ParseOptions};
use std::collections::{BTreeMap, HashMap};

/// Converts the provided Markdown string to BBCode suitable for Godot's docs renderer.
/// Simulates any missing features (e.g. tables) with a best-effort approach.
pub fn to_bbcode(md_text: &str) -> String {
    // to_mdast() never errors with normal Markdown, so unwrap is safe.
    let root = to_mdast(md_text, &ParseOptions::gfm()).unwrap();

    // Collect link/image definitions (for reference-style links).
    let definitions = root
        .children()
        .expect("Markdown root node should always have children")
        .iter()
        .filter_map(|node| match node {
            md::Node::Definition(def) => Some((&*def.identifier, &*def.url)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    // Convert the root node to BBCode.
    let mut converter = BBCodeConverter::new(&definitions);
    let content = converter.walk_node(&root, 0).unwrap_or_default();

    // Append footnotes at the bottom if any.
    if !converter.footnote_defs.is_empty() {
        let notes = converter
            .footnote_defs
            .iter()
            .map(|(idx, text)| format!("{} {}", BBCodeConverter::superscript(*idx), text))
            .collect::<Vec<_>>()
            .join("[br]");
        format!("{content}[br][br]{notes}")
    } else {
        content
    }
}

/// Manages the context needed to convert Markdown AST to Godot-compatible BBCode.
pub struct BBCodeConverter<'a> {
    /// Link/image definitions from the Markdown AST.
    definitions: &'a HashMap<&'a str, &'a str>,

    /// Footnote label -> numeric index.
    footnote_map: HashMap<String, usize>,

    /// Footnotes (index -> rendered text), sorted by index.
    footnote_defs: BTreeMap<usize, String>,

    /// Next footnote index to assign.
    current_footnote_index: usize,
}

// Given a Vec of Strings, if the Vec is empty, return None. Otherwise, join the strings
// with a separator and return the result.
fn join_if_not_empty(strings: Vec<String>, sep: &str) -> Option<String> {
    if strings.is_empty() {
        None
    } else {
        Some(strings.join(sep))
    }
}

impl<'a> BBCodeConverter<'a> {
    /// Creates a new converter with the provided link/image definitions.
    pub fn new(definitions: &'a HashMap<&'a str, &'a str>) -> Self {
        Self {
            definitions,
            footnote_map: HashMap::new(),
            footnote_defs: BTreeMap::new(),
            current_footnote_index: 1,
        }
    }

    /// Walk an AST node and return its BBCode. Returns `None` if the node should be
    /// ignored.
    ///
    /// `level` is used for nesting (e.g. lists).
    pub fn walk_node(&mut self, node: &md::Node, level: usize) -> Option<String> {
        use md::Node::*;

        let result = match node {
            // Root node: treat children as top-level blocks.
            // We join each block with [br][br], a double line break.
            Root(root) => {
                let block_strs: Vec<_> = root
                    .children
                    .iter()
                    .filter_map(|child| self.walk_node(child, level))
                    .collect();

                join_if_not_empty(block_strs, "[br][br]")?
            }

            // Paragraph: gather inline children as a single line.
            Paragraph(par) => self.walk_inline_nodes(&par.children, level),

            // Inline code -> [code]...[/code]
            InlineCode(md::InlineCode { value, .. }) => format!("[code]{value}[/code]"),

            // Strikethrough -> [s]...[/s]
            Delete(del) => {
                let inner = self.walk_inline_nodes(&del.children, level);
                format!("[s]{inner}[/s]")
            }

            // Italic -> [i]...[/i]
            Emphasis(emp) => {
                let inner = self.walk_inline_nodes(&emp.children, level);
                format!("[i]{inner}[/i]")
            }

            // Bold -> [b]...[/b]
            Strong(str) => {
                let inner = self.walk_inline_nodes(&str.children, level);
                format!("[b]{inner}[/b]")
            }

            // Plain text -> just the text, with newlines replaced by spaces.
            Text(txt) => txt.value.replace("\n", " "),

            // Heading -> single line, "fake" heading with [b]...[/b]
            Heading(h) => {
                let inner = self.walk_inline_nodes(&h.children, level);
                format!("[b]{inner}[/b]")
            }

            // Blockquote -> each child is effectively a block. We gather them with a single
            // [br] in between, then prefix each resulting line with "> ".
            Blockquote(bq) => {
                let child_blocks: Vec<_> = bq
                    .children
                    .iter()
                    .filter_map(|child| self.walk_node(child, level))
                    .collect();
                let content = child_blocks.join("[br]"); // Each child is a block.

                // Prefix each line with "> ".
                let mut out = String::new();
                for (i, line) in content.split("[br]").enumerate() {
                    if i > 0 {
                        out.push_str("[br]");
                    }
                    out.push_str("> ");
                    out.push_str(line);
                }
                out
            }

            // Code block -> [codeblock lang=??]...[/codeblock]
            Code(md::Code { value, lang, .. }) => {
                let maybe_lang = lang
                    .as_ref()
                    .map(|l| format!(" lang={}", l))
                    .unwrap_or_default();
                format!("[codeblock{maybe_lang}]{value}[/codeblock]")
            }

            // List -> each item is on its own line with indentation.
            // For ordered lists, we use a counter we increment for each item.
            // For unordered lists, we use '•'.
            List(lst) => {
                let indent = " ".repeat(level * 4);
                let is_ordered = lst.ordered;
                let mut counter = lst.start.unwrap_or(1);

                let mut lines = Vec::new();
                for item_node in &lst.children {
                    if let md::Node::ListItem(item) = item_node {
                        // Converts the item's children. These may be paragraphs or sub-lists, etc.
                        // We join multiple paragraphs in the same item with [br].
                        let item_str = self.walk_nodes_as_block(&item.children, level + 1);
                        let bullet = if is_ordered {
                            let b = format!("{}.", counter);
                            counter += 1;
                            b
                        } else {
                            "•".to_string()
                        };
                        let checkbox = match item.checked {
                            Some(true) => "[x] ",
                            Some(false) => "[ ] ",
                            None => "",
                        };

                        lines.push(format!("{indent}{bullet} {checkbox}{item_str}"));
                    }
                }

                join_if_not_empty(lines, "[br]")?
            }

            // Footnote reference -> a superscript number.
            FootnoteReference(fref) => {
                if let Some(label) = &fref.label {
                    let idx = *self.footnote_map.entry(label.clone()).or_insert_with(|| {
                        let i = self.current_footnote_index;
                        self.current_footnote_index += 1;
                        i
                    });
                    Self::superscript(idx)
                } else {
                    return None;
                }
            }

            // Footnote definition -> keep track of it, but produce no output here.
            FootnoteDefinition(fdef) => {
                if let Some(label) = &fdef.label {
                    let idx = *self.footnote_map.entry(label.clone()).or_insert_with(|| {
                        let i = self.current_footnote_index;
                        self.current_footnote_index += 1;
                        i
                    });
                    let def_content = self.walk_nodes_as_block(&fdef.children, level);
                    self.footnote_defs.insert(idx, def_content);
                }

                return None;
            }

            // Image -> [url=URL]URL[/url]
            Image(md::Image { url, .. }) => format!("[url={url}]{url}[/url]"),

            // Reference-style image -> [url=URL]URL[/url]
            ImageReference(img_ref) => {
                let url = self.definitions.get(&*img_ref.identifier).unwrap_or(&"");
                format!("[url={url}]{url}[/url]")
            }

            // Explicit link -> [url=URL]...[/url]
            Link(md::Link { url, children, .. }) => {
                let inner = self.walk_inline_nodes(children, level);
                format!("[url={url}]{inner}[/url]")
            }

            // Reference-style link -> [url=URL]...[/url]
            LinkReference(md::LinkReference {
                identifier,
                children,
                ..
            }) => {
                let url = self.definitions.get(&**identifier).unwrap_or(&"");
                let inner = self.walk_inline_nodes(children, level);
                format!("[url={url}]{inner}[/url]")
            }

            // Table: approximate by reading rows as block lines.
            Table(tbl) => {
                let rows: Vec<String> = tbl
                    .children
                    .iter()
                    .filter_map(|row| self.walk_node(row, level))
                    .collect();

                join_if_not_empty(rows, "[br]")?
            }

            // TableRow -> gather cells separated by " | "
            md::Node::TableRow(row) => {
                let cells: Vec<String> = row
                    .children
                    .iter()
                    .filter_map(|cell| self.walk_node(cell, level))
                    .collect();
                cells.join(" | ")
            }

            // TableCell -> treat as inline.
            md::Node::TableCell(cell) => self.walk_inline_nodes(&cell.children, level),

            // Raw HTML -> output as-is.
            Html(html) => html.value.clone(),

            // Hard line break -> single line break, with indentation if needed.
            Break(_) => format!("[br]{}", " ".repeat(level * 4)),

            // Fallback: just walk children.
            _ => {
                let children = node.children()?;
                self.walk_inline_nodes(children, level)
            }
        };

        Some(result)
    }

    /// Collects multiple sibling nodes that might be block-level (list items, etc.),
    /// joining them with `[br]`. Ignores nodes that return `None`. If all nodes return
    /// `None`, returns an empty string, as if the block was empty, since this function
    /// is called when we expect a block of content, even if it's empty.
    fn walk_nodes_as_block(&mut self, nodes: &[md::Node], level: usize) -> String {
        let mut pieces = Vec::new();
        for node in nodes {
            if let Some(s) = self.walk_node(node, level) {
                pieces.push(s);
            }
        }
        pieces.join("[br]")
    }

    /// Gathers children as an inline sequence: no forced breaks between them. Ignores
    /// nodes that return `None`. If all nodes return `None`, returns an empty string,
    /// as if the block was empty, since this function is called when we expect a block
    /// of content, even if it's empty.
    fn walk_inline_nodes(&mut self, children: &[md::Node], level: usize) -> String {
        let mut out = String::new();
        for child in children {
            if let Some(s) = self.walk_node(child, level) {
                out.push_str(&s);
            }
        }
        out
    }

    /// Convert a numeric index into a Unicode superscript (e.g. 123 -> ¹²³).
    pub fn superscript(idx: usize) -> String {
        const SUPS: &[char] = &['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
        idx.to_string()
            .chars()
            .filter_map(|c| c.to_digit(10).map(|d| SUPS[d as usize]))
            .collect()
    }
}
