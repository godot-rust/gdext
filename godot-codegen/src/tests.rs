/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Tests translation of certain symbols.
// See also integration tests: itest/engine_tests/codegen_[enums_]test.rs.

use crate::conv;
use crate::generator::native_structures::{parse_native_structures_format, NativeStructuresField};

#[test]
fn test_pascal_conversion() {
    // More in line with Rust identifiers, and eases recognition of other automation (like enumerator mapping).
    #[rustfmt::skip]
    let mappings = [
                                 ("AABB", "Aabb"),
                           ("AESContext", "AesContext"),
                              ("AStar3D", "AStar3D"),
                      ("AudioEffectEQ21", "AudioEffectEq21"),
                       ("AudioStreamWAV", "AudioStreamWav"),
                      ("CharFXTransform", "CharFxTransform"),
                       ("CPUParticles3D", "CpuParticles3D"),
              ("EditorSceneImporterGLTF", "EditorSceneImporterGltf"),
                              ("GIProbe", "GiProbe"),
                          ("HMACContext", "HmacContext"),
                           ("HSeparator", "HSeparator"),
                                   ("IP", "Ip"),
                         ("JNISingleton", "JniSingleton"),
                                 ("JSON", "Json"),
                      ("JSONParseResult", "JsonParseResult"),
                              ("JSONRPC", "JsonRpc"),
             ("NetworkedMultiplayerENet", "NetworkedMultiplayerENet"),
                             ("ObjectID", "ObjectId"),
                   ("OpenXRAPIExtension", "OpenXrApiExtension"),
                      ("OpenXRIPBinding", "OpenXrIpBinding"),
                   ("PackedFloat32Array", "PackedFloat32Array"),
                            ("PCKPacker", "PckPacker"),
                     ("PHashTranslation", "PHashTranslation"),
    ("PhysicsServer2DExtensionRayResult", "PhysicsServer2DExtensionRayResult"),
                                ("Rect2", "Rect2"),
                               ("Rect2i", "Rect2i"),
                                  ("RID", "Rid"),
                        ("StreamPeerSSL", "StreamPeerSsl"),
                          ("Transform3D", "Transform3D"),
                ("ViewportScreenSpaceAA", "ViewportScreenSpaceAa"),
                     ("ViewportSDFScale", "ViewportSdfScale"),
         ("WebRTCPeerConnectionGDNative", "WebRtcPeerConnectionGDNative"),
                      ("X509Certificate", "X509Certificate"),
                             ("XRServer", "XrServer"),
                                ("YSort", "YSort"),
    ];

    for (class_name, expected) in mappings {
        let actual = conv::to_pascal_case(class_name);
        assert_eq!(actual, expected, "PascalCase: ident `{class_name}`");
    }
}

#[test]
fn test_snake_conversion() {
    // More in line with Rust identifiers, and eases recognition of other automation (like enumerator mapping).
    #[rustfmt::skip]
    let mappings = [
                                 ("AABB", "aabb"),
                           ("AESContext", "aes_context"),
                              ("AStar3D", "a_star_3d"),
                      ("AudioEffectEQ21", "audio_effect_eq21"),
                       ("AudioStreamWAV", "audio_stream_wav"),
                      ("CharFXTransform", "char_fx_transform"),
                       ("CPUParticles3D", "cpu_particles_3d"),
              ("EditorSceneImporterGLTF", "editor_scene_importer_gltf"),
                              ("GIProbe", "gi_probe"),
                          ("HMACContext", "hmac_context"),
                           ("HSeparator", "h_separator"),
                                   ("IP", "ip"),
                         ("JNISingleton", "jni_singleton"),
                                 ("JSON", "json"),
                      ("JSONParseResult", "json_parse_result"),
                              ("JSONRPC", "json_rpc"),
             ("NetworkedMultiplayerENet", "networked_multiplayer_enet"),
                             ("ObjectID", "object_id"),
                   ("OpenXRAPIExtension", "open_xr_api_extension"),
                      ("OpenXRIPBinding", "open_xr_ip_binding"),
                   ("PackedFloat32Array", "packed_float32_array"),
                            ("PCKPacker", "pck_packer"),
                     ("PHashTranslation", "p_hash_translation"),
    ("PhysicsServer2DExtensionRayResult", "physics_server_2d_extension_ray_result"),
                                ("Rect2", "rect2"),
                               ("Rect2i", "rect2i"),
                                  ("RID", "rid"),
                        ("StreamPeerSSL", "stream_peer_ssl"),
                          ("Transform3D", "transform_3d"),
                ("ViewportScreenSpaceAA", "viewport_screen_space_aa"),
                     ("ViewportSDFScale", "viewport_sdf_scale"),
         ("WebRTCPeerConnectionGDNative", "web_rtc_peer_connection_gdnative"),
                      ("X509Certificate", "x509_certificate"),
                             ("XRServer", "xr_server"),
                                ("YSort", "y_sort"),

        // Enum names
                        ("AfterGUIInput", "after_gui_input"),
                           ("ASTCFormat", "astc_format"),
              ("Camera2DProcessCallback", "camera_2d_process_callback"),
                             ("FilterDB", "filter_db"),
                   ("G6DOFJointAxisFlag", "g6dof_joint_axis_flag"),
                               ("GIMode", "gi_mode"),
                                 ("MSAA", "msaa"),
                          ("SDFGIYScale", "sdfgi_y_scale"),
                         ("ViewportMSAA", "viewport_msaa"),
                              ("VRSMode", "vrs_mode"),
                            ("VSyncMode", "vsync_mode"),
    ];

    for (class_name, expected) in mappings {
        let actual = conv::to_snake_case(class_name);
        assert_eq!(actual, expected, "snake_case: ident `{class_name}`");
    }
}

#[test]
fn test_parse_native_structures_format() {
    // Convenience constructor.
    fn native(ty: &str, name: &str) -> NativeStructuresField {
        NativeStructuresField {
            field_type: String::from(ty),
            field_name: String::from(name),
            array_size: None,
        }
    }

    fn native_array(ty: &str, name: &str, array_size: usize) -> NativeStructuresField {
        NativeStructuresField {
            field_type: String::from(ty),
            field_name: String::from(name),
            array_size: Some(array_size),
        }
    }

    assert_eq!(parse_native_structures_format("").unwrap(), vec![]);

    // Check that we handle pointers correctly.
    assert_eq!(
        parse_native_structures_format("Object *a").unwrap(),
        vec![native("Object*", "a"),],
    );

    // Check that we deal with default values correctly. (We currently
    // strip and ignore them)
    assert_eq!(
        parse_native_structures_format("int x = 0").unwrap(),
        vec![native("int", "x"),],
    );

    let actual = parse_native_structures_format(
        "Vector3 position;Vector3 normal[5];Vector3 collider_velocity;Vector3 collider_angular_velocity;real_t depth;int local_shape;ObjectID collider_id;RID collider;int collider_shape"
    );
    let expected = vec![
        native("Vector3", "position"),
        native_array("Vector3", "normal", 5),
        native("Vector3", "collider_velocity"),
        native("Vector3", "collider_angular_velocity"),
        native("real_t", "depth"),
        native("int", "local_shape"),
        native("ObjectID", "collider_id"),
        native("RID", "collider"),
        native("int", "collider_shape"),
    ];
    assert_eq!(actual.unwrap(), expected);
}
