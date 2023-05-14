/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{
    parse_native_structures_format, to_pascal_case, to_snake_case, NativeStructuresField,
};

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
        let actual = to_pascal_case(class_name);
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
             ("NetworkedMultiplayerENet", "networked_multiplayer_e_net"),
                             ("ObjectID", "object_id"),
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
    ];

    for (class_name, expected) in mappings {
        let actual = to_snake_case(class_name);
        assert_eq!(actual, expected, "snake_case: ident `{class_name}`");
    }
}

#[test]
#[ignore] // enable once implemented
fn test_enumerator_names() {
    // How to deal with a naming convention that has evolved over a decade :)
    #[rustfmt::skip]
    let _mappings = [
        // No changes
        ("ModeFlags",                  "READ_WRITE",                          "READ_WRITE"),

        // Remove entire enum name
        ("SystemDir",                  "SYSTEM_DIR_DCIM",                     "DCIM"),
        ("Month",                      "MONTH_FEBRUARY",                      "FEBRUARY"),
        ("ProcessMode",                "PROCESS_MODE_WHEN_PAUSED",            "WHEN_PAUSED"),
        ("BodyMode",                   "BODY_MODE_KINEMATIC",                 "KINEMATIC"),
        ("GenEditState",               "GEN_EDIT_STATE_DISABLED",             "DISABLED"),
        ("JointType",                  "JOINT_TYPE_PIN",                      "PIN"),
        ("RenderingInfo",              "RENDERING_INFO_BUFFER_MEM_USED",      "BUFFER_MEM_USED"),
        ("CacheMode",                  "CACHE_MODE_IGNORE",                   "IGNORE"),

        // Remove entire name, but MiXED case
        ("VoxelGIQuality",             "VOXEL_GI_QUALITY_LOW",                "LOW"),
        ("CCDMode",                    "CCD_MODE_CAST_RAY",                   "CAST_RAY"),

        // Entire enum name, but changed
        ("Parameter",                  "PARAM_INITIAL_LINEAR_VELOCITY",       "INITIAL_LINEAR_VELOCITY"),
        ("SpaceParameter",             "SPACE_PARAM_CONTACT_MAX_SEPARATION",  "MAX_SEPARATION"),
        ("AreaParameter",              "AREA_PARAM_GRAVITY",                  "GRAVITY"),
        ("StencilOperation",           "STENCIL_OP_KEEP",                     "KEEP"),
        ("CompareOperator",            "COMPARE_OP_LESS",                     "LESS"),
        ("CubeMapLayer",               "CUBEMAP_LAYER_RIGHT",                 "RIGHT"),

        // Prefix omitted
        ("ProcessInfo",                "INFO_COLLISION_PAIRS",                "COLLISION_PAIRS"),
        ("PipelineDynamicStateFlags",  "DYNAMIC_STATE_DEPTH_BIAS",            "DEPTH_BIAS"),

        // Plural
        ("Hands",                      "HAND_LEFT",                           "LEFT"),
        ("Features",                   "FEATURE_SHADERS",                     "SHADERS"),

        // Unrelated name
        ("GlobalShaderParameterType",  "GLOBAL_VAR_TYPE_BOOL",                "BOOL"),

        // Implicitly used class name instead of enum name (OpenXRAction)
        ("ActionType",                 "OPENXR_ACTION_POSE",                  "POSE"),

        // Remove postfix (Mode, Type, Flags, Param, ...)
        ("CompressionMode",            "COMPRESSION_DEFLATE",                 "DEFLATE"),
        ("AreaSpaceOverrideMode",      "AREA_SPACE_OVERRIDE_COMBINE",         "COMBINE"),
        ("ProjectionType",             "PROJECTION_ORTHOGONAL",               "ORTHOGONAL"),
        ("ConnectFlags",               "CONNECT_PERSIST",                     "PERSIST"),
        ("ParticleFlags",              "PARTICLE_FLAG_ROTATE_Y",              "ROTATE_Y"),
        ("G6DOFJointAxisParam",        "G6DOF_JOINT_LINEAR_LOWER_LIMIT",      "LINEAR_LOWER_LIMIT"),
        ("MultimeshTransformFormat",   "MULTIMESH_TRANSFORM_3D",              "3D"),
        ("ThreadLoadStatus",           "THREAD_LOAD_INVALID_RESOURCE",        "INVALID_RESOURCE"),


        /*
        // Not handled:
        ("ActionType", "PROJECTION_ORTHOGONAL", "ORTHOGONAL", "OpenXRAction") // last = class name
        */
    ];
}

#[test]
fn test_parse_native_structures_format() {
    // Convenience constructor.
    fn native(ty: &str, name: &str) -> NativeStructuresField {
        NativeStructuresField {
            field_type: String::from(ty),
            field_name: String::from(name),
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

    assert_eq!(
    parse_native_structures_format("Vector3 position;Vector3 normal;Vector3 collider_velocity;Vector3 collider_angular_velocity;real_t depth;int local_shape;ObjectID collider_id;RID collider;int collider_shape").unwrap(),
    vec![
      native("Vector3", "position"),
      native("Vector3", "normal"),
      native("Vector3", "collider_velocity"),
      native("Vector3", "collider_angular_velocity"),
      native("real_t", "depth"),
      native("int", "local_shape"),
      native("ObjectID", "collider_id"),
      native("RID", "collider"),
      native("int", "collider_shape"),
    ],
  );
}
