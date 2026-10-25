//! WaterDropEngine's `wde-ui` crate: an ECS-driven game UI, laid out with [taffy](https://github.com/DioxusLabs/taffy).
//!
//! A UI is a hierarchy of entities with a [`UiNode`](node::UiNode), whose [`UiStyle`](style::UiStyle) follows the CSS
//! flexbox / grid / block model. Nodes are drawn as rounded rectangles on top of the scene with a
//! [`UiBackground`](node::UiBackground), [`UiBorderColor`](node::UiBorderColor) and
//! [`UiCornerRadius`](node::UiCornerRadius), and react to the pointer with a
//! [`UiInteraction`](interaction::UiInteraction). A [`UiText`](text::UiText) node is sized to its text,
//! shaped with cosmic-text and drawn with glyphon.
//!
//! # Example
//! ```rust,ignore
//! commands.spawn((
//!     UiNode(UiStyle {
//!         width: Val::Percent(100.0),
//!         height: Val::Px(48.0),
//!         padding: UiRect::all(Val::Px(8.0)),
//!         ..default()
//!     }),
//!     UiBackground(Color::from_srgba(0.1, 0.1, 0.1, 0.8)),
//!     children![(
//!         UiNode(UiStyle { width: Val::Px(32.0), height: Val::Px(32.0), ..default() }),
//!         UiBackground(Color::WHITE),
//!         UiInteraction::default()
//!     )]
//! ));
//! ```
//!
//! Each frame in `PostUpdate` ([`UiSystems`]), the hierarchy is laid out against the window and the
//! visible nodes are uploaded to the GPU, then drawn by the [`UiRenderPass`].
use bevy::{input::InputSystems, prelude::*};
use wde_renderer::prelude::*;

use crate::{
    interaction::{UiClick, UiPointer, update_interactions},
    layout::{UiLayoutTree, compute_layout, sync_layout_tree},
    render::{
        data::{UiQuadData, UiQuadsBinding},
        extract::{UiDrawList, build_draw_list, update_ui_buffer},
        image::{UiDrawBatches, UiImageBindGroups, UiImageLayout},
        pass::SubRenderPassUi,
        pipeline::UiRenderPipeline,
        text::{SubRenderPassUiText, UiTextRenderer, build_text_draws, prepare_text}
    },
    text::UiFonts
};

pub mod interaction;
mod layout;
pub mod node;
mod render;
pub mod style;
pub mod text;

pub use render::pass::UiRenderPass;

#[doc(hidden)]
pub mod prelude {
    pub use crate::interaction::{UiClick, UiInteraction, UiPointer};
    pub use crate::node::{
        ComputedUiNode, UiBackground, UiBorderColor, UiCornerRadius, UiImage, UiNode
    };
    pub use crate::style::*;
    pub use crate::text::UiText;
    pub use crate::{UiPlugin, UiSystems};
}

/// Lays out the UI and builds its draw list, in `PostUpdate`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiSystems;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let fonts = UiFonts::default();
        app.insert_resource(fonts.clone())
            .init_resource::<UiLayoutTree>()
            .init_resource::<UiPointer>()
            .init_resource::<UiDrawList>()
            .add_message::<UiClick>()
            .add_systems(PreUpdate, update_interactions.after(InputSystems))
            .add_systems(
                PostUpdate,
                (
                    sync_layout_tree,
                    compute_layout,
                    (build_draw_list, build_text_draws).chain()
                )
                    .chain()
                    .in_set(UiSystems)
            );

        app.add_plugins((
            ExtractResourcePlugin::<UiDrawList>::default(),
            RenderDataRegisterPlugin::<UiQuadData>::default(),
            RenderBindingRegisterPlugin::<UiQuadsBinding>::default(),
            RenderPipelineRegisterPlugin::<UiRenderPipeline>::default()
        ));

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .insert_resource(fonts)
            .init_resource::<UiDrawList>()
            .init_resource::<UiDrawBatches>()
            .init_resource::<UiImageBindGroups>()
            .init_resource::<UiImageLayout>()
            .add_systems(
                Render,
                (update_ui_buffer, prepare_text).in_set(RenderSet::Prepare)
            );
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_pass::<UiRenderPass>()
            .add_sub_pass::<SubRenderPassUi, UiRenderPass>()
            .add_sub_pass::<SubRenderPassUiText, UiRenderPass>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        let text_renderer = UiTextRenderer::new(render_app.world());
        render_app.insert_resource(text_renderer);
    }
}
