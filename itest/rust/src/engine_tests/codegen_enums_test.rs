/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Tests the presence, naming and accessibility of generated enum and enumerator symbols.
// Only enabled in full codegen mode (including experimental APIs).

#![cfg(feature = "codegen-full-experimental")]

use crate::framework::itest;

use godot::engine::audio_effect_spectrum_analyzer::FftSize;
use godot::engine::base_material_3d::Flags;
use godot::engine::camera_2d::Camera2DProcessCallback;
use godot::engine::camera_3d::ProjectionType;
use godot::engine::cpu_particles_2d::{Parameter, ParticleFlags};
use godot::engine::editor_plugin::CustomControlContainer;
use godot::engine::environment::SdfgiYScale;
use godot::engine::file_access::{CompressionMode, ModeFlags};
use godot::engine::http_client::ResponseCode;
use godot::engine::image::Format;
use godot::engine::mesh::ArrayType;
use godot::engine::navigation_path_query_parameters_2d::PathPostProcessing;
use godot::engine::node::ProcessMode;
use godot::engine::object::ConnectFlags;
use godot::engine::open_xr_action::ActionType;
use godot::engine::open_xr_hand::Hands;
use godot::engine::open_xr_interface::HandJointFlags;
use godot::engine::os::SystemDir;
use godot::engine::physical_bone_3d::JointType;
use godot::engine::physics_server_2d::{AreaParameter, BodyMode, CcdMode};
use godot::engine::physics_server_3d::{
    AreaSpaceOverrideMode, G6dofJointAxisParam, ProcessInfo, SpaceParameter,
};
use godot::engine::rendering_device::{
    CompareOperator, PipelineDynamicStateFlags, StencilOperation,
};
use godot::engine::rendering_server::{
    ArrayFormat, CubeMapLayer, EnvironmentSdfgiYScale, EnvironmentSsaoQuality, Features,
    GlobalShaderParameterType, MultimeshTransformFormat, RenderingInfo, ViewportScaling3DMode,
    VoxelGiQuality,
};
use godot::engine::resource_format_loader::CacheMode;
use godot::engine::resource_loader::ThreadLoadStatus;
use godot::engine::rigid_body_2d::CenterOfMassMode;
use godot::engine::scene_state::GenEditState;
use godot::engine::shader::Mode;
use godot::engine::sub_viewport::UpdateMode;
use godot::engine::time::Month;
use godot::engine::upnp::UpnpResult;
use godot::engine::viewport::Msaa;
use godot::engine::visual_shader_node_float_op::Operator;
use godot::engine::visual_shader_node_vector_func::Function;
use godot::engine::voxel_gi::Subdiv;
use godot::engine::xr_interface::{EnvironmentBlendMode, TrackingStatus};
use godot::engine::xr_pose::TrackingConfidence;
use godot::engine::zip_packer::ZipAppend;

#[itest]
fn codegen_enums_exist() {
    // Remove entire enum name.
    let _ = ModeFlags::READ_WRITE;
    let _ = BodyMode::KINEMATIC;
    let _ = CacheMode::IGNORE;
    let _ = CenterOfMassMode::AUTO;
    let _ = Format::RF;
    let _ = GenEditState::DISABLED;
    let _ = JointType::PIN;
    let _ = Mode::SKY;
    let _ = Month::FEBRUARY;
    let _ = ProcessMode::WHEN_PAUSED;
    let _ = RenderingInfo::BUFFER_MEM_USED;
    let _ = SystemDir::DCIM;

    // Remove entire name, but MiXED case.
    let _ = VoxelGiQuality::LOW;
    let _ = CcdMode::CAST_RAY;
    let _ = UpnpResult::HTTP_ERROR;
    let _ = SdfgiYScale::SCALE_100_PERCENT;
    let _ = EnvironmentSdfgiYScale::SCALE_50_PERCENT;

    // Entire enum name, but changed.
    let _ = Parameter::INITIAL_LINEAR_VELOCITY;
    let _ = SpaceParameter::CONTACT_MAX_SEPARATION;
    let _ = AreaParameter::GRAVITY;
    let _ = StencilOperation::KEEP;
    let _ = CompareOperator::LESS;
    let _ = CubeMapLayer::RIGHT;
    let _ = Camera2DProcessCallback::PHYSICS;

    // Prefix omitted.
    let _ = ArrayType::CUSTOM0;
    let _ = PathPostProcessing::EDGECENTERED;
    let _ = PipelineDynamicStateFlags::DEPTH_BIAS;
    let _ = ProcessInfo::COLLISION_PAIRS;
    let _ = ResponseCode::NO_CONTENT;
    let _ = UpdateMode::WHEN_VISIBLE;
    let _ = ZipAppend::CREATE;

    // Plural.
    let _ = Hands::LEFT;
    let _ = Features::SHADERS;
    let _ = Flags::ALBEDO_TEXTURE_FORCE_SRGB;

    // Unrelated name.
    let _ = GlobalShaderParameterType::BOOL;
    let _ = ArrayFormat::FLAG_FORMAT_VERSION_2;
    let _ = CustomControlContainer::CANVAS_EDITOR_SIDE_LEFT;

    // Implicitly used class name instead of enum name (OpenXR*, XR*).
    let _ = ActionType::POSE;
    let _ = TrackingConfidence::NONE;
    let _ = TrackingStatus::NOT_TRACKING;
    let _ = EnvironmentBlendMode::OPAQUE;

    // Abbreviation.
    let _ = Operator::ATAN2;
    let _ = Function::LOG;
    let _ = EnvironmentSsaoQuality::HIGH;

    // Remove postfix (Mode, Type, Flags, Param, ...).
    let _ = CompressionMode::DEFLATE;
    let _ = AreaSpaceOverrideMode::COMBINE;
    let _ = ProjectionType::ORTHOGONAL;
    let _ = ConnectFlags::PERSIST;
    let _ = HandJointFlags::ORIENTATION_TRACKED;
    let _ = ParticleFlags::ROTATE_Y;
    let _ = G6dofJointAxisParam::LINEAR_LOWER_LIMIT;
    let _ = ThreadLoadStatus::INVALID_RESOURCE;
    let _ = ViewportScaling3DMode::BILINEAR;

    // Remaining identifier is non-valid (digit).
    let _ = Subdiv::SUBDIV_64;
    let _ = FftSize::SIZE_256;
    let _ = Msaa::MSAA_8X;
    let _ = MultimeshTransformFormat::TRANSFORM_3D;
}
