//! Mirrors the [`UiNode`] hierarchy into a taffy tree and writes the layout back to [`ComputedUiNode`].
use std::sync::{Arc, MutexGuard};
use wde_logger::prelude::*;

use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    window::PrimaryWindow
};
use glyphon::{Buffer, FontSystem};
use taffy::{AvailableSpace, NodeId, TaffyTree};

use crate::{
    node::{ComputedUiNode, UiNode},
    text::{UiFonts, UiText, UiTextBuffer, measure, shape}
};

#[derive(Resource)]
pub(crate) struct UiLayoutTree {
    taffy: TaffyTree<UiTextBuffer>,
    nodes: EntityHashMap<NodeId>
}
// SAFETY: taffy lengths are tagged pointers only to hold `calc()` values, which `UiStyle::to_taffy` never creates.
unsafe impl Send for UiLayoutTree {}
unsafe impl Sync for UiLayoutTree {}
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Buffer>();
};

impl Default for UiLayoutTree {
    fn default() -> Self {
        Self {
            taffy: TaffyTree::new(),
            nodes: EntityHashMap::default()
        }
    }
}
impl UiLayoutTree {
    pub(crate) fn text_buffer(&self, entity: Entity) -> Option<&Arc<Buffer>> {
        let id = self.nodes.get(&entity)?;
        self.taffy.get_node_context(*id).map(|text| &text.0)
    }
}

/// Locks the font system on first use, as the render world uses it concurrently.
struct LazyFontSystem<'a> {
    fonts: &'a UiFonts,
    guard: Option<MutexGuard<'a, FontSystem>>
}
impl<'a> LazyFontSystem<'a> {
    fn new(fonts: &'a UiFonts) -> Self {
        Self { fonts, guard: None }
    }
    fn get(&mut self) -> &mut FontSystem {
        self.guard
            .get_or_insert_with(|| self.fonts.0.lock().unwrap())
    }
}

/// Creates, updates and removes taffy nodes, and re-links the children of the nodes whose UI
/// children may have changed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_layout_tree(
    mut tree: ResMut<UiLayoutTree>,
    fonts: Res<UiFonts>,
    mut removed: RemovedComponents<UiNode>,
    mut removed_texts: RemovedComponents<UiText>,
    changed: Query<(Entity, &UiNode), Changed<UiNode>>,
    changed_texts: Query<(Entity, &UiText), Changed<UiText>>,
    added: Query<&ChildOf, Added<UiNode>>,
    changed_children: Query<Entity, (With<UiNode>, Changed<Children>)>,
    children: Query<&Children>
) {
    let tree = &mut *tree;
    for entity in removed.read() {
        if let Some(node) = tree.nodes.remove(&entity) {
            let _ = tree.taffy.remove(node);
        }
    }

    for (entity, node) in &changed {
        let result = match tree.nodes.get(&entity) {
            Some(&id) => tree.taffy.set_style(id, node.0.to_taffy()),
            None => tree.taffy.new_leaf(node.0.to_taffy()).map(|id| {
                tree.nodes.insert(entity, id);
            })
        };
        if let Err(e) = result {
            error!("Failed to update UI node {}: {}.", entity, e);
        }
    }

    let mut font_system = LazyFontSystem::new(&fonts);
    let texts = removed_texts.read().map(|entity| (entity, None)).chain(
        changed_texts
            .iter()
            .map(|(entity, text)| (entity, Some(text)))
    );
    for (entity, text) in texts {
        if let Some(&id) = tree.nodes.get(&entity) {
            let buffer = text.map(|text| shape(font_system.get(), text));
            let _ = tree.taffy.set_node_context(id, buffer);
        }
    }

    let dirty_parents: EntityHashSet = changed_children
        .iter()
        .chain(added.iter().map(ChildOf::parent))
        .collect();
    for parent in dirty_parents {
        let Some(&parent_id) = tree.nodes.get(&parent) else {
            continue;
        };
        let child_ids: Vec<NodeId> = children
            .get(parent)
            .into_iter()
            .flatten()
            .filter_map(|child| tree.nodes.get(child).copied())
            .collect();
        if let Err(e) = tree.taffy.set_children(parent_id, &child_ids) {
            error!("Failed to update UI node {} children: {}.", parent, e);
        }
    }
}

/// Lays out each root against the window, then writes absolute positions and draw order to the
/// [`ComputedUiNode`]s, in depth-first order.
pub(crate) fn compute_layout(
    mut tree: ResMut<UiLayoutTree>,
    fonts: Res<UiFonts>,
    window: Single<&Window, With<PrimaryWindow>>,
    nodes: Query<(Entity, Option<&ChildOf>), With<UiNode>>,
    children: Query<&'static Children>,
    computed: Query<&'static mut ComputedUiNode>
) {
    let available = taffy::Size {
        width: AvailableSpace::Definite(window.width()),
        height: AvailableSpace::Definite(window.height())
    };
    let mut roots: Vec<Entity> = nodes
        .iter()
        .filter(|(_, parent)| parent.is_none_or(|p| !nodes.contains(p.parent())))
        .map(|(entity, _)| entity)
        .collect();
    roots.sort();

    let mut writer = LayoutWriter {
        font_system: LazyFontSystem::new(&fonts),
        children: &children,
        computed,
        order: 0
    };
    for root in roots {
        let Some(&root_id) = tree.nodes.get(&root) else {
            continue;
        };
        // taffy only hides `display: none` children, not roots
        let hidden = tree
            .taffy
            .style(root_id)
            .is_ok_and(|style| style.display == taffy::Display::None);
        if !hidden {
            let font_system = &mut writer.font_system;
            let result = tree.taffy.compute_layout_with_measure(
                root_id,
                available,
                |inputs, _, text, style| {
                    taffy::compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |known, available| {
                            let Some(UiTextBuffer(buffer)) = text else {
                                return taffy::Size::ZERO;
                            };
                            let width = known.width.or(match available.width {
                                AvailableSpace::MinContent => Some(0.0),
                                AvailableSpace::MaxContent => None,
                                AvailableSpace::Definite(width) => Some(width)
                            });
                            let size = measure(font_system.get(), buffer, width);
                            taffy::Size {
                                width: known.width.unwrap_or(size.x),
                                height: known.height.unwrap_or(size.y)
                            }
                        }
                    )
                }
            );
            if let Err(e) = result {
                error!("Failed to compute UI layout of {}: {}.", root, e);
                continue;
            }
        }
        writer.write(&mut tree, root, Vec2::ZERO, hidden);
    }
}

/// Writes the layout of a hierarchy to its [`ComputedUiNode`]s, in depth-first order.
struct LayoutWriter<'a, 'w, 's> {
    font_system: LazyFontSystem<'a>,
    children: &'a Query<'a, 'a, &'static Children>,
    computed: Query<'w, 's, &'static mut ComputedUiNode>,
    order: u32
}
impl LayoutWriter<'_, '_, '_> {
    fn write(
        &mut self,
        tree: &mut UiLayoutTree,
        entity: Entity,
        parent_position: Vec2,
        hidden: bool
    ) {
        let (Some(&id), Ok(mut node)) = (tree.nodes.get(&entity), self.computed.get_mut(entity))
        else {
            return;
        };
        let Ok(&layout) = tree.taffy.layout(id) else {
            return;
        };
        let position = parent_position + Vec2::new(layout.location.x, layout.location.y);
        let edges =
            |rect: taffy::Rect<f32>| Vec4::new(rect.left, rect.top, rect.right, rect.bottom);
        node.set_if_neq(ComputedUiNode {
            position,
            size: match hidden {
                true => Vec2::ZERO,
                false => Vec2::new(layout.size.width, layout.size.height)
            },
            border: edges(layout.border),
            padding: edges(layout.padding),
            order: self.order
        });
        self.order += 1;

        // Wrap the text to its final width. The unrounded one avoids wrapping a line that measured
        // slightly wider than its rounded box. The slack absorbs float errors.
        let unrounded = *tree.taffy.unrounded_layout(id);
        if let Some(UiTextBuffer(buffer)) = tree.taffy.get_node_context_mut(id)
            && !hidden
        {
            let content_width = unrounded.content_box_width();
            measure(self.font_system.get(), buffer, Some(content_width + 0.5));
        }

        for &child in self.children.get(entity).into_iter().flatten() {
            self.write(tree, child, position, hidden);
        }
    }
}
