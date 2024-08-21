/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Converts [Markdown](https://en.wikipedia.org/wiki/Markdown) to [BBCode](https://en.wikipedia.org/wiki/BBCode).

use markdown::mdast::Node;
use markdown::{to_mdast, ParseOptions};
use std::collections::HashMap;

pub fn to_bbcode(md: &str) -> String {
    // to_mdast() never errors with normal arkdown, so unwrap is safe.
    let n = to_mdast(md, &ParseOptions::gfm()).unwrap();
    let definitions = n
        .children()
        .unwrap() // root node always has children
        .iter()
        .filter_map(|n| match n {
            Node::Definition(definition) => Some((&*definition.identifier, &*definition.url)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    walk_node(&n, &definitions).unwrap_or_default()
}

fn walk_node(node: &Node, definitions: &HashMap<&str, &str>) -> Option<String> {
    use Node::*;
    let bbcode = match node {
        Root(root) => walk_nodes(&root.children, definitions, "[br][br]"),
        InlineCode(markdown::mdast::InlineCode { value, .. }) => format!("[code]{value}[/code]"),
        Delete(delete) => format!("[s]{}[/s]", walk_nodes(&delete.children, definitions, "")),
        Emphasis(emphasis) => format!("[i]{}[/i]", walk_nodes(&emphasis.children, definitions, "")),
        Image(markdown::mdast::Image { url, .. }) => format!("[img]{url}[/img]",),
        ImageReference(image) => {
            format!(
                "[img]{}[/img]",
                definitions.get(&&*image.identifier).unwrap()
            )
        }
        Link(markdown::mdast::Link { url, children, .. }) => {
            format!("[url={url}]{}[/url]", walk_nodes(children, definitions, ""))
        }
        LinkReference(markdown::mdast::LinkReference {
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
        Code(markdown::mdast::Code { value, .. }) => format!("[codeblock]{value}[/codeblock]"),
        Paragraph(paragraph) => walk_nodes(&paragraph.children, definitions, ""),
        // bbcode supports lists but docs dont
        List(_) | BlockQuote(_) | FootnoteReference(_) | FootnoteDefinition(_) | Table(_) => {
            "".into()
        }
        Html(html) => html.value.clone(),
        _ => walk_nodes(&node.children()?, definitions, ""),
    };
    Some(bbcode)
}

/// Calls [`walk_node`] over every node its given, joining them with the supplied separator.
fn walk_nodes(nodes: &[Node], definitions: &HashMap<&str, &str>, separator: &str) -> String {
    nodes
        .iter()
        .filter_map(|n| walk_node(n, definitions))
        .collect::<Vec<_>>()
        .join(separator)
}
