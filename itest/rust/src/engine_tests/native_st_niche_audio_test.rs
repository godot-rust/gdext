/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Requires BOTH full codegen and experimental-threads. The latter would compile without, but crash at runtime.
// More tests on native structures are in native_structure_full_codegen_tests.rs.
#![cfg(all(feature = "codegen-full", feature = "experimental-threads"))]

use std::thread;
use std::time::Duration;

use godot::builtin::Vector2;
use godot::classes::native::AudioFrame;
use godot::classes::{
    AudioEffect, AudioEffectInstance, AudioServer, AudioStreamGenerator,
    AudioStreamGeneratorPlayback, AudioStreamPlayer, Engine, IAudioEffect, IAudioEffectInstance,
    SceneTree,
};
use godot::obj::{Base, Gd, NewAlloc, NewGd, Singleton};
use godot::register::{godot_api, GodotClass};

use crate::framework::itest;

#[derive(GodotClass)]
#[class(base = AudioEffect, init)]
struct AudioEffectReceiver {
    base: Base<AudioEffect>,
}

#[godot_api]
impl IAudioEffect for AudioEffectReceiver {
    fn instantiate(&mut self) -> Option<Gd<AudioEffectInstance>> {
        Some(AudioEffectReceiverInstance::new_gd().upcast())
    }
}

#[derive(GodotClass)]
#[class(base = AudioEffectInstance, init)]
struct AudioEffectReceiverInstance {
    was_called: bool,
    base: Base<AudioEffectInstance>,
}

#[godot_api]
impl IAudioEffectInstance for AudioEffectReceiverInstance {
    unsafe fn process_rawptr(
        &mut self,
        _src_buffer: *const std::ffi::c_void,
        dst_buffer: *mut AudioFrame,
        _frame_count: i32,
    ) {
        (*dst_buffer).left = 15.0;
        (*dst_buffer).right = -12.0;
        self.was_called = true;
    }
}

#[derive(GodotClass)]
#[class(base = AudioEffect, init)]
struct AudioEffectAsserter {
    base: Base<AudioEffect>,
}

#[godot_api]
impl IAudioEffect for AudioEffectAsserter {
    fn instantiate(&mut self) -> Option<Gd<AudioEffectInstance>> {
        Some(AudioEffectAsserterInstance::new_gd().upcast())
    }
}

#[derive(GodotClass)]
#[class(base = AudioEffectInstance, init)]
struct AudioEffectAsserterInstance {
    was_called: bool,
    base: Base<AudioEffectInstance>,
}

#[godot_api]
impl IAudioEffectInstance for AudioEffectAsserterInstance {
    unsafe fn process_rawptr(
        &mut self,
        src_buffer: *const std::ffi::c_void,
        _dst_buffer: *mut AudioFrame,
        _frame_count: i32,
    ) {
        let src = src_buffer as *const AudioFrame;

        assert_eq!((*src).left, 15.0);
        assert_eq!((*src).right, -12.0);

        self.was_called = true;
    }
}

#[itest]
fn native_audio_structure_out_parameter() {
    // Create two audio effects: One that writes to the out parameter and one that reads the incomming parameter.
    let receiver = AudioEffectReceiver::new_gd().upcast::<AudioEffect>();
    let asserter = AudioEffectAsserter::new_gd().upcast::<AudioEffect>();

    let mut audio_server = AudioServer::singleton();

    // Add both effects to the audio bus so they get called during audio processing.
    audio_server.add_bus_effect(0, &receiver);
    audio_server.add_bus_effect(0, &asserter);

    let generator = AudioStreamGenerator::new_gd();
    let mut player = AudioStreamPlayer::new_alloc();

    let tree = Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>();

    tree.get_root().unwrap().add_child(&player);
    player.set_stream(&generator);

    // Start playback so we can push audio frames through the audio pipeline.
    player.play();

    let mut playback = player
        .get_stream_playback()
        .unwrap()
        .cast::<AudioStreamGeneratorPlayback>();

    let length = playback.get_frames_available();

    if length == 0 {
        panic!("must be able to push at least one frame!");
    }

    // Create dummy audio frame.
    playback.push_frame(Vector2::ONE);
    thread::sleep(Duration::from_secs(1));

    // stop playback to cleanup playback instance
    player.stop();
    player.free();

    // Verify that both audio effects were called.
    let receiver_instance = audio_server
        .get_bus_effect_instance(0, 0)
        .unwrap()
        .cast::<AudioEffectReceiverInstance>();
    let asserter_instance = audio_server
        .get_bus_effect_instance(0, 1)
        .unwrap()
        .cast::<AudioEffectAsserterInstance>();

    assert!(receiver_instance.bind().was_called);
    assert!(asserter_instance.bind().was_called);
}
