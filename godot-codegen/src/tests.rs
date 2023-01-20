/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::to_module_name;

#[test]
fn module_name_generator() {
    let tests = vec![
        // A number of test cases to cover some possibilities:
        // * Underscores are removed
        // * First character is always lowercased
        // * lowercase to an uppercase inserts an underscore
        //   - FooBar => foo_bar
        // * two capital letter words does not separate the capital letters:
        //   - FooBBaz => foo_bbaz (lower, cap, cap, lower)
        // * many-capital letters to lowercase inserts an underscore before the last uppercase letter:
        //   - FOOBar => boo_bar
        // underscores
        ("Ab_Cdefg", "ab_cdefg"),
        ("_Abcd", "abcd"),
        ("Abcd_", "abcd"),
        // first and last
        ("Abcdefg", "abcdefg"),
        ("abcdefG", "abcdef_g"),
        // more than 2 caps
        ("ABCDefg", "abc_defg"),
        ("AbcDEFg", "abc_de_fg"),
        ("AbcdEF10", "abcd_ef10"),
        ("AbcDEFG", "abc_defg"),
        ("ABCDEFG", "abcdefg"),
        ("ABC", "abc"),
        // Lowercase to an uppercase
        ("AbcDefg", "abc_defg"),
        // Only 2 caps
        ("ABcdefg", "abcdefg"),
        ("ABcde2G", "abcde_2g"),
        ("AbcDEfg", "abc_defg"),
        ("ABcDe2G", "abc_de_2g"),
        ("abcdeFG", "abcde_fg"),
        ("AB", "ab"),
        // Lowercase to an uppercase
        ("AbcdefG", "abcdef_g"), // PosX => pos_x
        // text changes
        ("FooVec3Uni", "foo_vec3_uni"),
        ("GDExtension", "gdextension_"),
        ("GDScript", "gdscript"),
    ];
    tests.iter().for_each(|(class_name, expected)| {
        let actual = to_module_name(class_name);
        assert_eq!(*expected, actual, "Input: {class_name}");
    });
}

#[test]
fn test_name_smoother() {
    // More in line with Rust identifiers, and eases recognition of other automation (like enumerator mapping).
    #[rustfmt::skip]
    let _mappings = [
        ("RID",                    "Rid"),
        ("AESContext",             "AesContext"),
        ("AudioEffectEQ21",        "AudioEffectEq21"),
        ("AudioStreamWAV",         "AudioStreamWav"),
        ("CPUParticles3D",         "CpuParticles3D"),
        ("ClassDB",                "ClassDb"),               // should multi-uppercase at the end be retained?
        ("CharFXTransform",        "CharFxTransform"),
        ("ViewportSDFScale",       "ViewportSdfScale"),
        ("ViewportMSAA",           "ViewportMsaa"),
        ("ViewportScreenSpaceAA",  "ViewportScreenSpaceAa"),

        // unchanged
        ("AStar3D",                "AStar3D"),
    ];
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
