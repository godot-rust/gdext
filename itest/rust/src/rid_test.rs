/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{collections::HashSet, thread};

use godot::{
    engine::RenderingServer,
    prelude::{inner::InnerRid, Color, Rid, Vector2},
};

use crate::{itest, suppress_godot_print};

#[itest]
fn rid_equiv() {
    let invalid: Rid = Rid::Invalid;
    let valid: Rid = Rid::new((10 << 32) | 20);
    assert!(!InnerRid::from_outer(&invalid).is_valid());
    assert!(InnerRid::from_outer(&valid).is_valid());

    assert_eq!(InnerRid::from_outer(&invalid).get_id(), 0);
    assert_eq!(InnerRid::from_outer(&valid).get_id(), (10 << 32) | 20);
}

#[itest]
fn canvas_set_parent() {
    // This originally caused UB, but still testing it here in case it breaks.
    let mut server = RenderingServer::singleton();
    let canvas = server.canvas_create();
    let viewport = server.viewport_create();

    suppress_godot_print(|| server.canvas_item_set_parent(viewport, canvas));
    suppress_godot_print(|| server.canvas_item_set_parent(viewport, viewport));

    server.free_rid(canvas);
    server.free_rid(viewport);
}

#[itest]
fn multi_thread_test() {
    let threads = (0..10)
        .map(|_| {
            thread::spawn(|| {
                let mut server = RenderingServer::singleton();
                (0..1000).map(|_| server.canvas_item_create()).collect()
            })
        })
        .collect::<Vec<_>>();

    let mut rids: Vec<Rid> = vec![];

    for thread in threads.into_iter() {
        rids.append(&mut thread.join().unwrap());
    }

    let set = rids.iter().cloned().collect::<HashSet<_>>();
    assert_eq!(set.len(), rids.len());

    let mut server = RenderingServer::singleton();

    for rid in rids.iter() {
        server.canvas_item_add_circle(*rid, Vector2::ZERO, 1.0, Color::from_rgb(1.0, 0.0, 0.0));
    }

    for rid in rids.iter() {
        server.free_rid(*rid);
    }
}

/// Check that godot does not crash upon receiving various RIDs that may be edge cases. As it could do in Godot 3.
#[itest]
fn strange_rids() {
    let mut server = RenderingServer::singleton();
    let mut rids: Vec<u64> = vec![
        // Invalid RID.
        0,
        // Normal RID, should work without issue.
        1,
        10,
        // Testing the boundaries of various ints.
        u8::MAX as u64,
        u16::MAX as u64,
        u32::MAX as u64,
        u64::MAX,
        i8::MIN as u64,
        i8::MAX as u64,
        i16::MIN as u64,
        i16::MAX as u64,
        i32::MIN as u64,
        i32::MAX as u64,
        i64::MIN as u64,
        i64::MAX as u64,
        // Biggest RIDs possible in Godot (ignoring local indices).
        0xFFFFFFFF << 32,
        0x7FFFFFFF << 32,
        // Godot's servers treats RIDs as two u32s, so testing what happens round the region where
        // one u32 overflows into the next.
        u32::MAX as u64 + 1,
        u32::MAX as u64 + 2,
        u32::MAX as u64 - 1,
        u32::MAX as u64 - 2,
        // A couple random RIDs.
        1234567891011121314,
        14930753991246632225,
        8079365198791785081,
        10737267678893224303,
        12442588258967011829,
        4275912429544145425,
    ];
    // Checking every number with exactly 2 bits = 1.
    // An approximation of exhaustively checking every number.
    for i in 0..64 {
        for j in 0..63 {
            if j >= i {
                rids.push((1 << i) | (1 << (j + 1)))
            } else {
                rids.push((1 << i) | (1 << j))
            }
        }
    }

    for id in rids.iter() {
        suppress_godot_print(|| server.canvas_item_clear(Rid::new(*id)))
    }
}
