/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::conv;
use crate::util::{parse_native_structures_format, NativeStructuresField};

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
             ("NetworkedMultiplayerENet", "networked_multiplayer_e_net"),
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
fn test_enumerator_names() {
    // How to deal with a naming convention that has evolved over a decade :)
    #[rustfmt::skip]
    let mappings = [
        // No changes
        ("ModeFlags",                  "READ_WRITE",                          "READ_WRITE"),

        // Remove entire enum name
        ("BodyMode",                   "BODY_MODE_KINEMATIC",                 "KINEMATIC"),
        ("CacheMode",                  "CACHE_MODE_IGNORE",                   "IGNORE"),
        ("CenterOfMassMode",           "CENTER_OF_MASS_MODE_AUTO",            "AUTO"),
        ("Format",                     "FORMAT_RF",                           "RF"),
        ("GenEditState",               "GEN_EDIT_STATE_DISABLED",             "DISABLED"),
        ("JointType",                  "JOINT_TYPE_PIN",                      "PIN"),
        ("Mode",                       "MODE_SKY",                            "SKY"),
        ("Month",                      "MONTH_FEBRUARY",                      "FEBRUARY"),
        ("ProcessMode",                "PROCESS_MODE_WHEN_PAUSED",            "WHEN_PAUSED"),
        ("RenderingInfo",              "RENDERING_INFO_BUFFER_MEM_USED",      "BUFFER_MEM_USED"),
        ("SystemDir",                  "SYSTEM_DIR_DCIM",                     "DCIM"),

        // Remove entire name, but MiXED case
        ("VoxelGIQuality",             "VOXEL_GI_QUALITY_LOW",                "LOW"),
        ("CCDMode",                    "CCD_MODE_CAST_RAY",                   "CAST_RAY"),
        ("UPNPResult",                 "UPNP_RESULT_HTTP_ERROR",              "HTTP_ERROR"),
        ("SDFGIYScale",                "SDFGI_Y_SCALE_100_PERCENT",           "SCALE_100_PERCENT"),
        ("EnvironmentSDFGIYScale",     "ENV_SDFGI_Y_SCALE_50_PERCENT",        "SCALE_50_PERCENT"),

        // Entire enum name, but changed
        ("Parameter",                  "PARAM_INITIAL_LINEAR_VELOCITY",       "INITIAL_LINEAR_VELOCITY"),
        ("SpaceParameter",             "SPACE_PARAM_CONTACT_MAX_SEPARATION",  "CONTACT_MAX_SEPARATION"),
        ("AreaParameter",              "AREA_PARAM_GRAVITY",                  "GRAVITY"),
        ("StencilOperation",           "STENCIL_OP_KEEP",                     "KEEP"),
        ("CompareOperator",            "COMPARE_OP_LESS",                     "LESS"),
        ("CubeMapLayer",               "CUBEMAP_LAYER_RIGHT",                 "RIGHT"),
        ("Camera2DProcessCallback",    "CAMERA2D_PROCESS_PHYSICS",            "PHYSICS"),

        // Prefix omitted
        ("ArrayType",                  "ARRAY_CUSTOM0",                       "CUSTOM0"),
        ("PathPostProcessing",         "PATH_POSTPROCESSING_EDGECENTERED",    "EDGECENTERED"),
        ("PipelineDynamicStateFlags",  "DYNAMIC_STATE_DEPTH_BIAS",            "DEPTH_BIAS"),
        ("ProcessInfo",                "INFO_COLLISION_PAIRS",                "COLLISION_PAIRS"),
        ("ResponseCode",               "RESPONSE_NO_CONTENT",                 "NO_CONTENT"),
        ("UpdateMode",                 "UPDATE_WHEN_VISIBLE",                 "WHEN_VISIBLE"),
        ("ZipAppend",                  "APPEND_CREATE",                       "CREATE"),

        // Plural
        ("Hands",                      "HAND_LEFT",                           "LEFT"),
        ("Features",                   "FEATURE_SHADERS",                     "SHADERS"),
        ("Flags",                      "FLAG_ALBEDO_TEXTURE_FORCE_SRGB",      "ALBEDO_TEXTURE_FORCE_SRGB"),

        // Unrelated name
        ("GlobalShaderParameterType",  "GLOBAL_VAR_TYPE_BOOL",                "BOOL"),
        ("ArrayFormat",                "ARRAY_FLAG_FORMAT_VERSION_2",         "VERSION_2"),
        ("CustomControlContainer",     "CONTAINER_CANVAS_EDITOR_SIDE_LEFT",   "CANVAS_EDITOR_SIDE_LEFT"),

        // Implicitly used class name instead of enum name (OpenXR*, XR*)
        ("ActionType",                 "OPENXR_ACTION_POSE",                  "POSE"), // class OpenXRAction
        ("TrackingConfidence",         "XR_TRACKING_CONFIDENCE_NONE",         "NONE"), // class XRPose
        ("TrackingStatus",             "XR_NOT_TRACKING",                     "NOT_TRACKING"), // class XRInterface
        ("EnvironmentBlendMode",       "XR_ENV_BLEND_MODE_OPAQUE",            "OPAQUE"),

        // Abbreviation
        ("Operator",                   "OP_ATAN2",                            "ATAN2"), // class "VisualShaderNodeVectorOp"
        ("Function",                   "FUNC_LOG",                            "LOG"),
        ("EnvironmentSSAOQuality",     "ENV_SSAO_QUALITY_HIGH",               "HIGH"),

        // Remove postfix (Mode, Type, Flags, Param, ...)
        ("CompressionMode",            "COMPRESSION_DEFLATE",                 "DEFLATE"),
        ("AreaSpaceOverrideMode",      "AREA_SPACE_OVERRIDE_COMBINE",         "COMBINE"),
        ("ProjectionType",             "PROJECTION_ORTHOGONAL",               "ORTHOGONAL"),
        ("ConnectFlags",               "CONNECT_PERSIST",                     "PERSIST"),
        ("HandJointFlags",             "HAND_JOINT_ORIENTATION_TRACKED",      "ORIENTATION_TRACKED"),
        ("ParticleFlags",              "PARTICLE_FLAG_ROTATE_Y",              "ROTATE_Y"),
        ("G6DOFJointAxisParam",        "G6DOF_JOINT_LINEAR_LOWER_LIMIT",      "LINEAR_LOWER_LIMIT"),
        ("ThreadLoadStatus",           "THREAD_LOAD_INVALID_RESOURCE",        "INVALID_RESOURCE"),
        ("ViewportScaling3DMode",      "VIEWPORT_SCALING_3D_MODE_BILINEAR",   "BILINEAR"),

        // Remaining identifier is non-valid
        ("Subdiv",                     "SUBDIV_64",                           "SUBDIV_64"),
        ("FFTSize",                    "FFT_SIZE_256",                        "SIZE_256"),
        ("MSAA",                       "MSAA_8X",                             "MSAA_8X"),
        ("MultimeshTransformFormat",   "MULTIMESH_TRANSFORM_3D",              "TRANSFORM_3D"),

        // Test cases that are not perfect but accepted as they are.
        //("SpecialHistory",             "REMOTE_HISTORY",                      "REMOTE_HISTORY"),
        //("Mode",                       "CONVEX_DECOMPOSITION_MODE_VOXEL",     "VOXEL"), // class ConvexDecompositionSettings
    ];

    for (enum_name, enumerator_name, expected) in mappings {
        let mapped_enum_name = conv::to_pascal_case(enum_name);
        let actual = conv::make_enumerator_name(enumerator_name, &mapped_enum_name);

        assert_eq!(
            actual, expected,
            "enumerator `{enum_name}.{enumerator_name}`"
        );
    }
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

    let actual = parse_native_structures_format(
        "Vector3 position;Vector3 normal;Vector3 collider_velocity;Vector3 collider_angular_velocity;real_t depth;int local_shape;ObjectID collider_id;RID collider;int collider_shape"
    );
    let expected = vec![
        native("Vector3", "position"),
        native("Vector3", "normal"),
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
