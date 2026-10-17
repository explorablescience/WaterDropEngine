use bevy::prelude::{Quat, Transform, Vec3};
use std::collections::HashMap;
use wde_animation::{AnimationClip, Interpolation, Keyframes};
use wde_logger::prelude::*;

use crate::accessor::parse_attribute_as_f32;
use crate::error::GltfError;
use crate::material::GltfMaterial;
use crate::model::{
    AccessorData, BufferSliceData, GltfAccessorComponentType, GltfBone, GltfBuffer, GltfLight,
    GltfLightKind, GltfMesh, GltfModel, MeshPrimitive, ModelCondition
};
use base64::{Engine, engine::general_purpose};
use serde_json::Value;

/// Prefix marking a glTF mesh as a named property (e.g. a spawn point) rather than a
/// renderable mesh. Its position and scale are recorded but it is otherwise skipped.
const PROPERTY_MESH_PREFIX: &str = "%P%";
/// Prefix marking a node as a bone of a rigged model: its mesh stays in the bone's space, and
/// animations move it at runtime. Only bones may have children, which must be bones too.
const BONE_NODE_PREFIX: &str = "%B%";
/// Range of the lights without one, in world units.
const DEFAULT_LIGHT_RANGE: f32 = 6.0;

/// Parse a glTF 2.0 JSON file from `res/` and build an in-memory `GltfModel`.
pub fn parse_gltf(bytes: Vec<u8>, path: &str) -> Result<GltfModel, GltfError> {
    // Extract filename and folder path
    let std_path = std::path::Path::new(path);
    let filename = std_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unnamed.gltf")
        .to_string();
    let folder_path = std_path
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string();
    trace!("Parsing glTF file '{}'.", path);

    // Parse the JSON
    let json: Value = serde_json::from_slice(&bytes)?;

    // Assert asset version
    let asset = &json["asset"];
    let version = asset["version"]
        .as_str()
        .ok_or(GltfError::MissingField("asset.version".to_string()))?;
    if version != "2.0" {
        return Err(GltfError::UnsupportedVersion(version.to_string()));
    }

    // Read buffers
    let buffers = json["buffers"]
        .as_array()
        .ok_or(GltfError::MissingField("buffers".to_string()))?;
    if buffers.is_empty() {
        return Err(GltfError::NoBuffers);
    } else if buffers.len() > 1 {
        return Err(GltfError::MultipleBuffers(buffers.len()));
    }
    let single_buffer = &buffers[0];
    let buffer_uri = single_buffer["uri"]
        .as_str()
        .ok_or(GltfError::MissingField("buffer.uri".to_string()))?;

    // Decode buffer data
    let buffer_data: Vec<u8> = if buffer_uri.starts_with("data:application/octet-stream;base64,") {
        let base64_data = buffer_uri
            .strip_prefix("data:application/octet-stream;base64,")
            .ok_or(GltfError::Base64Error("Invalid data URI".to_string()))?;
        general_purpose::STANDARD.decode(base64_data)?
    } else {
        let buffer_path = std::path::Path::new(&folder_path).join(buffer_uri);
        std::fs::read(format!("./res/{}", buffer_path.display()))?
    };

    // Read which accessors correspond to which buffer views
    let accessors = json["accessors"]
        .as_array()
        .ok_or(GltfError::MissingField("accessors".to_string()))?;

    // Read buffer views
    let buffer_views = json["bufferViews"]
        .as_array()
        .ok_or(GltfError::MissingField("bufferViews".to_string()))?;
    trace!(
        "Found {} bufferViews and {} accessors",
        buffer_views.len(),
        accessors.len()
    );

    let mut slices_data = Vec::new();
    for (i, buffer_view) in buffer_views.iter().enumerate() {
        // For simplicity, we only handle buffer index 0 in this example
        let buffer_index = buffer_view["buffer"]
            .as_i64()
            .ok_or(GltfError::MissingField(format!("bufferView[{}].buffer", i)))?;
        if buffer_index != 0 {
            return Err(GltfError::MultipleBuffers(buffer_index as usize + 1));
        }

        // Get byte offset and length for this view
        let byte_offset = buffer_view
            .get("byteOffset")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let _byte_length = buffer_view["byteLength"]
            .as_i64()
            .ok_or(GltfError::MissingField(format!(
                "bufferView[{}].byteLength",
                i
            )))?;

        // Store buffer slice data
        let slice_data = BufferSliceData {
            byte_offset: byte_offset as usize
        };
        slices_data.push(slice_data);
    }
    let buffer = GltfBuffer {
        data: buffer_data,
        slices: slices_data
    };

    // Retrieve the nodes of the default scene
    let scene_index = json["scene"]
        .as_i64()
        .ok_or(GltfError::MissingField("scene".to_string()))?;
    let nodes_list = json["scenes"][scene_index as usize]["nodes"]
        .as_array()
        .ok_or(GltfError::MissingField("scenes.nodes".to_string()))?;
    trace!("Processing {} nodes", nodes_list.len());

    // Handle each node, depth first so that parent bones come before their children
    let mut mesh_primitives: Vec<MeshPrimitive> = Vec::new();
    let mut properties: HashMap<String, (Vec3, Vec3)> = HashMap::new();
    let mut material_datas: Vec<GltfMaterial> = Vec::new();
    let mut material_map: HashMap<i64, usize> = HashMap::new();
    let mut bones: Vec<GltfBone> = Vec::new();
    let mut node_bones: HashMap<usize, usize> = HashMap::new();
    let mut lights: Vec<GltfLight> = Vec::new();
    let mut stack: Vec<(usize, Option<usize>)> = nodes_list
        .iter()
        .rev()
        .map(|node| {
            node.as_u64()
                .map(|idx| (idx as usize, None))
                .ok_or(GltfError::JsonError("Invalid node index".to_string()))
        })
        .collect::<Result<_, _>>()?;
    while let Some((node_idx, parent_bone)) = stack.pop() {
        let node = &json["nodes"][node_idx];

        // Handle meshes in the node
        let mesh = node
            .get("mesh")
            .and_then(|v| v.as_i64())
            .map(|mesh_index| &json["meshes"][mesh_index as usize]);
        // Prefer the node's (object's) name over the mesh data-block's name: the `%P%`
        // prefix is set by renaming the object in Blender, which does not rename the
        // underlying mesh data if it is not unique to that node.
        let name = node
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| mesh.and_then(|m| m.get("name")).and_then(|v| v.as_str()))
            .unwrap_or("unnamed_mesh");

        // Extract node transform
        let translation = node
            .get("translation")
            .and_then(|v| v.as_array())
            .map(|arr| {
                [
                    arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32
                ]
            })
            .unwrap_or([0.0, 0.0, 0.0]);

        let rotation = node
            .get("rotation")
            .and_then(|v| v.as_array())
            .map(|arr| {
                [
                    arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    arr.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32
                ]
            })
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);

        let scale = node
            .get("scale")
            .and_then(|v| v.as_array())
            .map(|arr| {
                [
                    arr.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                    arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32
                ]
            })
            .unwrap_or([1.0, 1.0, 1.0]);

        // Light nodes
        if let Some(light) = node
            .pointer("/extensions/KHR_lights_punctual/light")
            .and_then(|v| v.as_u64())
        {
            let transform = Transform {
                translation: Vec3::from(translation),
                rotation: Quat::from_array(rotation),
                scale: Vec3::ONE
            };
            match parse_light(&json, light as usize, name, transform) {
                Some(light) => lights.push(light),
                None => warn!("Ignoring invalid light '{}' in '{}'.", name, path)
            }
            continue;
        }

        let children = node.get("children").and_then(|v| v.as_array());
        let bone = match name.strip_prefix(BONE_NODE_PREFIX) {
            Some(bone_name) => {
                let bone = bones.len();
                bones.push(GltfBone {
                    name: bone_name.to_string(),
                    parent: parent_bone,
                    rest: Transform {
                        translation: Vec3::from(translation),
                        rotation: Quat::from_array(rotation),
                        scale: Vec3::from(scale)
                    }
                });
                node_bones.insert(node_idx, bone);
                for child in children.into_iter().flatten().rev() {
                    if let Some(child) = child.as_u64() {
                        stack.push((child as usize, Some(bone)));
                    }
                }
                Some(bone)
            }
            None => {
                if children.is_some_and(|c| !c.is_empty()) {
                    warn!(
                        "Ignoring the children of node '{}' in '{}': only bone nodes ('{}') may have children.",
                        name, path, BONE_NODE_PREFIX
                    );
                }
                None
            }
        };
        let Some(mesh) = mesh else {
            match bone {
                Some(_) => continue, // Pivot without geometry
                None => return Err(GltfError::MissingField("node.mesh".to_string()))
            }
        };
        // Bone meshes stay in the bone's space
        let (translation, rotation, scale) = match bone {
            Some(_) => ([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3]),
            None => (translation, rotation, scale)
        };

        // Meshes prefixed with "%P%" are not real renderable meshes: they mark a named
        // property (e.g. a spawn point) and only their position/scale are kept.
        if let Some(property_name) = name.strip_prefix(PROPERTY_MESH_PREFIX) {
            properties.insert(
                property_name.to_string(),
                (Vec3::from(translation), Vec3::from(scale))
            );
            continue;
        }

        // Process primitives in the mesh (each primitive contains a description of how to draw a part of the mesh)
        let primitives = mesh["primitives"]
            .as_array()
            .ok_or(GltfError::MissingField("mesh.primitives".to_string()))?;
        for (i, primitive) in primitives.iter().enumerate() {
            // Process optional indexed vertices
            let vertex_indexed_accessor_id = primitive
                .get("indices")
                .and_then(|index_accessor| index_accessor.as_i64());
            let vertex_indexed_accessor = if let Some(index_accessor) = vertex_indexed_accessor_id {
                let accessor = &accessors[index_accessor as usize];

                // Build AccessorData
                let buffer_view_index = accessor["bufferView"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("accessor.bufferView".to_string()))?;
                let byte_offset = accessor
                    .get("byteOffset")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;
                let count = accessor["count"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("accessor.count".to_string()))?
                    as usize;
                let component_type_value =
                    accessor["componentType"]
                        .as_i64()
                        .ok_or(GltfError::MissingField(
                            "accessor.componentType".to_string()
                        ))?;
                let component_type = GltfAccessorComponentType::from_i64(component_type_value)
                    .ok_or(GltfError::UnsupportedComponentType(component_type_value))?;
                let min = accessor.get("min").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_f64().map(|f| f as f32))
                            .collect()
                    })
                });
                let max = accessor.get("max").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_f64().map(|f| f as f32))
                            .collect()
                    })
                });
                Some(AccessorData {
                    buffer_view_index,
                    byte_offset,
                    component_type,
                    count,
                    accessor_type: accessor["type"]
                        .as_str()
                        .expect("Accessor missing type")
                        .to_string(),
                    min,
                    max
                })
            } else {
                None
            };

            // Process vertex attributes
            let attributes = &primitive["attributes"];
            let mut vertex_attributes: Vec<(String, AccessorData)> = Vec::new();
            for (attr_name, accessor_value) in
                attributes.as_object().expect("Attributes is not an object")
            {
                // Retrieve accessor index
                let accessor_index = accessor_value
                    .as_i64()
                    .expect("Invalid accessor index for attribute");
                let accessor = &accessors[accessor_index as usize];

                // Build AccessorData
                let buffer_view_index = accessor["bufferView"]
                    .as_i64()
                    .expect("Accessor missing bufferView");
                let byte_offset = accessor
                    .get("byteOffset")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;
                let count = accessor["count"].as_i64().expect("Accessor missing count") as usize;
                let component_type_value = accessor["componentType"]
                    .as_i64()
                    .expect("Accessor missing componentType");
                let component_type = GltfAccessorComponentType::from_i64(component_type_value)
                    .expect("Unsupported accessor component type");
                let accessor_type = accessor["type"]
                    .as_str()
                    .expect("Accessor missing type")
                    .to_string();
                let min = accessor.get("min").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_f64().map(|f| f as f32))
                            .collect()
                    })
                });
                let max = accessor.get("max").and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_f64().map(|f| f as f32))
                            .collect()
                    })
                });
                let accessor_data = AccessorData {
                    buffer_view_index,
                    byte_offset,
                    component_type,
                    count,
                    accessor_type,
                    min,
                    max
                };
                vertex_attributes.push((attr_name.clone(), accessor_data));
            }

            // Process material (if any)
            let material_index = primitive.get("material").and_then(|v| v.as_i64());
            if let Some(mat_index) = material_index
                && !material_map.contains_key(&mat_index)
            {
                let mat_data = parse_material(
                    Some(mat_index),
                    &json["materials"][mat_index as usize],
                    &json,
                    &folder_path
                )?;
                if let Some(mat) = mat_data {
                    material_map.insert(mat_index, material_datas.len());
                    material_datas.push(mat);
                }
            }

            // Build MeshPrimitive
            let mesh_primitive = MeshPrimitive {
                name: format!("{}_primitive_{}", name, i),
                vertex_indexed: vertex_indexed_accessor,
                vertex_attributes,
                // Index into `material_datas` (first-use order), not the raw glTF material index
                material_id: material_index
                    .and_then(|idx| material_map.get(&idx))
                    .map(|&idx| idx as u32),
                translation,
                rotation,
                scale,
                condition: ModelCondition::from_node_name(name),
                bone
            };
            mesh_primitives.push(mesh_primitive);
        }
    }

    // Add the remaining materials that were not assigned to any mesh
    let raw_materials = json["materials"]
        .as_array()
        .ok_or(GltfError::MissingField("materials".to_string()))?;
    for (i, material_json) in raw_materials.iter().enumerate() {
        if !material_map.contains_key(&(i as i64)) {
            let mat_data = parse_material(Some(i as i64), material_json, &json, &folder_path)?;
            if let Some(mat) = mat_data {
                material_datas.push(mat);
            }
        }
    }

    let animations = parse_animations(&json, accessors, &buffer, &node_bones, &bones)?;

    // GltfModel construction
    Ok(GltfModel {
        filename,
        path: folder_path,
        buffers: vec![buffer],
        meshes: vec![GltfMesh {
            primitives: mesh_primitives
        }],
        materials: material_datas,
        properties,
        bones,
        animations,
        lights
    })
}

/// Parse a `KHR_lights_punctual` light, placed by its node transform.
fn parse_light(json: &Value, index: usize, name: &str, transform: Transform) -> Option<GltfLight> {
    let light = json.pointer(&format!("/extensions/KHR_lights_punctual/lights/{index}"))?;
    let number = |v: &Value| v.as_f64().map(|f| f as f32);
    let color = light
        .get("color")
        .and_then(|c| c.as_array())
        .and_then(|c| {
            let channel = |i: usize| c.get(i).and_then(number);
            Some(Vec3::new(channel(0)?, channel(1)?, channel(2)?))
        })
        .unwrap_or(Vec3::ONE);
    let kind = match light.get("type")?.as_str()? {
        "point" => GltfLightKind::Point,
        "spot" => GltfLightKind::Spot {
            inner_cone: light
                .pointer("/spot/innerConeAngle")
                .and_then(number)
                .unwrap_or(0.0),
            outer_cone: light
                .pointer("/spot/outerConeAngle")
                .and_then(number)
                .unwrap_or(std::f32::consts::FRAC_PI_4)
        },
        _ => return None
    };
    Some(GltfLight {
        name: name.to_string(),
        kind,
        transform,
        color,
        intensity: light.get("intensity").and_then(number).unwrap_or(1.0),
        range: light
            .get("range")
            .and_then(number)
            .unwrap_or(DEFAULT_LIGHT_RANGE)
    })
}

/// Parse the accessor JSON description.
fn parse_accessor(accessor: &Value) -> Result<AccessorData, GltfError> {
    let field = |name: &str| GltfError::MissingField(format!("accessor.{name}"));
    let component_type_value = accessor["componentType"]
        .as_i64()
        .ok_or(field("componentType"))?;
    let bounds = |key: &str| {
        accessor.get(key).and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|val| val.as_f64().map(|f| f as f32))
                .collect()
        })
    };
    Ok(AccessorData {
        buffer_view_index: accessor["bufferView"].as_i64().ok_or(field("bufferView"))?,
        byte_offset: accessor
            .get("byteOffset")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize,
        component_type: GltfAccessorComponentType::from_i64(component_type_value)
            .ok_or(GltfError::UnsupportedComponentType(component_type_value))?,
        count: accessor["count"].as_i64().ok_or(field("count"))? as usize,
        accessor_type: accessor["type"].as_str().ok_or(field("type"))?.to_string(),
        min: bounds("min"),
        max: bounds("max")
    })
}

/// Parse the glTF animations into clips, keeping the channels that target bones.
fn parse_animations(
    json: &Value,
    accessors: &[Value],
    buffer: &GltfBuffer,
    node_bones: &HashMap<usize, usize>,
    bones: &[GltfBone]
) -> Result<Vec<AnimationClip>, GltfError> {
    let animations = json.get("animations").and_then(|v| v.as_array());
    let mut clips = Vec::new();
    for (i, animation) in animations.into_iter().flatten().enumerate() {
        let mut clip = AnimationClip {
            name: animation
                .get("name")
                .and_then(|v| v.as_str())
                .map_or_else(|| format!("animation_{i}"), str::to_string),
            ground_speed: animation
                .pointer("/extras/ground_speed")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32),
            ..Default::default()
        };
        let samplers = animation["samplers"]
            .as_array()
            .ok_or(GltfError::MissingField("animation.samplers".to_string()))?;
        let channels = animation["channels"]
            .as_array()
            .ok_or(GltfError::MissingField("animation.channels".to_string()))?;
        for channel in channels {
            let target = &channel["target"];
            let path = target["path"].as_str().unwrap_or_default();
            let width = match path {
                "translation" | "scale" => 3,
                "rotation" => 4,
                _ => {
                    warn!(
                        "Ignoring unsupported '{}' channel of animation '{}'.",
                        path, clip.name
                    );
                    continue;
                }
            };
            let Some(&bone) = target["node"]
                .as_u64()
                .and_then(|node| node_bones.get(&(node as usize)))
            else {
                warn!(
                    "Ignoring a channel of animation '{}' on a node which is not a bone.",
                    clip.name
                );
                continue;
            };
            let sampler = channel["sampler"]
                .as_u64()
                .and_then(|s| samplers.get(s as usize))
                .ok_or(GltfError::MissingField(
                    "animation.channel.sampler".to_string()
                ))?;
            let read = |key: &str| -> Result<Vec<f32>, GltfError> {
                let accessor = sampler[key]
                    .as_u64()
                    .and_then(|a| accessors.get(a as usize))
                    .ok_or(GltfError::MissingField(format!("animation.sampler.{key}")))?;
                parse_attribute_as_f32(&parse_accessor(accessor)?, buffer)
            };
            let times = read("input")?;
            let mut values = read("output")?;
            let interpolation = match sampler.get("interpolation").and_then(|v| v.as_str()) {
                Some("STEP") => Interpolation::Step,
                Some("CUBICSPLINE") => {
                    // Keep the value of each (in-tangent, value, out-tangent) triplet
                    warn!(
                        "Animation '{}' uses cubic splines, approximated linearly.",
                        clip.name
                    );
                    values = values
                        .chunks_exact(width * 3)
                        .flat_map(|key| key[width..width * 2].to_vec())
                        .collect();
                    Interpolation::Linear
                }
                _ => Interpolation::Linear
            };
            if let Some(&last) = times.last() {
                clip.duration = clip.duration.max(last);
            }

            let track = clip.tracks.entry(bones[bone].name.clone()).or_default();
            let vec3s = |values: &[f32]| values.chunks_exact(3).map(Vec3::from_slice).collect();
            match path {
                "translation" => {
                    track.translation = Some(Keyframes {
                        times,
                        values: vec3s(&values),
                        interpolation
                    })
                }
                "rotation" => {
                    track.rotation = Some(Keyframes {
                        times,
                        values: values
                            .chunks_exact(4)
                            .map(|q| Quat::from_slice(q).normalize())
                            .collect(),
                        interpolation
                    })
                }
                _ => {
                    track.scale = Some(Keyframes {
                        times,
                        values: vec3s(&values),
                        interpolation
                    })
                }
            }
        }
        trace!("Loaded animation '{}' ({:.2}s).", clip.name, clip.duration);
        clips.push(clip);
    }
    Ok(clips)
}

/// Parse material data from the glTF JSON
fn parse_material(
    material_index: Option<i64>,
    material_json: &Value,
    json: &Value,
    folder_path: &str
) -> Result<Option<GltfMaterial>, GltfError> {
    if material_index.is_none() {
        return Ok(None);
    }
    let pbr = &material_json["pbrMetallicRoughness"];

    // Extract material number properties
    let base_color_factor = pbr["baseColorFactor"]
        .as_array()
        .unwrap_or(&vec![
            Value::from(1.0),
            Value::from(1.0),
            Value::from(1.0),
            Value::from(1.0),
        ])
        .iter()
        .map(|v| v.as_f64().unwrap_or(1.0) as f32)
        .collect::<Vec<f32>>();
    let metallic_factor = pbr["metallicFactor"].as_f64().unwrap_or(1.0) as f32;
    let roughness_factor = pbr["roughnessFactor"].as_f64().unwrap_or(1.0) as f32;

    // Extract material textures
    const OPT_TEX_CANDIDATES: [(&str, &str); 4] = [
        ("pbr", "baseColorTexture"),
        ("pbr", "metallicRoughnessTexture"),
        ("root", "normalTexture"),
        ("root", "occlusionTexture")
    ];
    let mut opt_textures_urls = HashMap::new();
    for tex_name in OPT_TEX_CANDIDATES {
        let try_get_image = match tex_name.0 {
            "pbr" => pbr.get(tex_name.1),
            "root" => material_json.get(tex_name.1),
            _ => None
        };
        if let Some(tex_info) = try_get_image {
            let tex_id = tex_info["index"].as_i64();

            // Retrieve texture data
            if let Some(tex_id) = tex_id {
                let texture = &json["textures"][tex_id as usize];
                let image_index = texture["source"]
                    .as_i64()
                    .ok_or(GltfError::MissingField("texture.source".to_string()))?;

                // Retrieve image URI
                let image = &json["images"][image_index as usize];
                let image_uri = image["uri"]
                    .as_str()
                    .ok_or(GltfError::MissingField("image.uri".to_string()))?
                    .to_string();
                opt_textures_urls.insert(tex_name.1.to_string(), image_uri);
            }
        }
    }

    // Build GltfMaterial
    Ok(Some(GltfMaterial {
        name: material_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("material_{}", material_index.unwrap()))
            .to_string(),
        folder_path: folder_path.to_string(),
        base_color: base_color_factor.try_into().unwrap_or([1.0, 1.0, 1.0, 1.0]),
        alpha_blend: material_json.get("alphaMode").and_then(|v| v.as_str()) == Some("BLEND"),
        metallic: metallic_factor,
        roughness: roughness_factor,
        base_color_tex_url: opt_textures_urls.get("baseColorTexture").cloned(),
        metallic_roughness_tex_url: opt_textures_urls.get("metallicRoughnessTexture").cloned(),
        normal_tex_url: opt_textures_urls.get("normalTexture").cloned(),
        occlusion_tex_url: opt_textures_urls.get("occlusionTexture").cloned()
    }))
}
