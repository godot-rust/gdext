/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Converts [Markdown](https://en.wikipedia.org/wiki/Markdown) to [BBCode](https://en.wikipedia.org/wiki/BBCode).

use markdown::mdast as md;
use markdown::{to_mdast, ParseOptions};
use std::collections::HashMap;

pub fn to_bbcode(md: &str) -> String {
    // to_mdast() never errors with normal Markdown, so unwrap is safe.
    let n = to_mdast(md, &ParseOptions::gfm()).unwrap();

    let definitions = n
        .children()
        .unwrap() // root node always has children
        .iter()
        .filter_map(|n| match n {
            md::Node::Definition(def) => Some((&*def.identifier, &*def.url)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    walk_node(&n, &definitions).unwrap_or_default()
}

fn walk_node(node: &md::Node, definitions: &HashMap<&str, &str>) -> Option<String> {
    use md::Node::*;

    let bbcode = match node {
        Root(root) => walk_nodes(&root.children, definitions, "[br][br]"),

        InlineCode(md::InlineCode { value, .. }) => format!("[code]{value}[/code]"),

        Delete(delete) => format!("[s]{}[/s]", walk_nodes(&delete.children, definitions, "")),

        Emphasis(emphasis) => format!("[i]{}[/i]", walk_nodes(&emphasis.children, definitions, "")),

        Image(md::Image { url, .. }) => format!("[img]{url}[/img]",),

        ImageReference(image) => {
            format!(
                "[img]{}[/img]",
                definitions.get(&&*image.identifier).unwrap()
            )
        }

        Link(md::Link { url, children, .. }) => {
            format!("[url={url}]{}[/url]", walk_nodes(children, definitions, ""))
        }

        LinkReference(md::LinkReference {
            identifier,
            children,
            ..
        }) => format!(
            "[url={}]{}[/url]",
            definitions.get(&&**identifier).unwrap(),
            walk_nodes(children, definitions, "")
        ),

        Strong(strong) => format!("[b]{}[/b]", walk_nodes(&strong.children, definitions, "")),

        Text(text) => text.value.clone(),

        // TODO: more langs?
        Code(md::Code { value, .. }) => format!("[codeblock]{value}[/codeblock]"),

        Paragraph(paragraph) => walk_nodes(&paragraph.children, definitions, ""),

        // BBCode supports lists, but docs don't.
        List(_) | Blockquote(_) | FootnoteReference(_) | FootnoteDefinition(_) | Table(_) => {
            String::new()
        }

        Html(html) => html.value.clone(),

        _ => walk_nodes(node.children()?, definitions, ""),
    };

    Some(bbcode)
}

/// Calls [`walk_node`] over every node it receives, joining them with the supplied separator.
fn walk_nodes(nodes: &[md::Node], definitions: &HashMap<&str, &str>, separator: &str) -> String {
    nodes
        .iter()
        .filter_map(|n| walk_node(n, definitions))
        .collect::<Vec<_>>()
        .join(separator)
}
