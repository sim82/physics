use std::any::Any;

use bevy::{
    ecs::{query::QueryEntityError, system::SystemParam},
    prelude::*,
};

use super::{
    components,
    undo::{self, UndoCommands, UndoStack},
};
use crate::csg;
use thiserror::Error;

pub mod add_brush;
pub mod add_pointlight;
pub mod duplicate_brush;
pub mod remove_entity;
pub mod set_brush_material;
pub mod update_brush_drag;
pub mod update_point_transform;

#[derive(Error, Debug)]
pub enum EditCommandError {
    #[error("Entity {0:?} is not available")]
    UnknownEntity(Entity),

    #[error("Entity query error {0:?}")]
    EntityQueryError(#[from] QueryEntityError),
}

// pub type Result<T> = std::result::Result<T, EditCommandError>;
pub use anyhow::Result;

pub trait UndoDowncast {
    fn as_any(&self) -> &dyn Any;
}

pub trait UndoCommand: UndoDowncast {
    fn try_merge(&mut self, other: &dyn UndoCommand) -> bool;
    fn undo(&self, undo_commands: &mut UndoCommands) -> Result<()>;
}

impl<T: Any> UndoDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait EditCommand {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>>;
}

pub mod prelude {
    pub use super::{EditCommand, EditCommandError, EditCommands, Result, UndoCommand};
    pub use crate::{
        csg,
        editor::{components, undo::UndoCommands},
    };
    pub use anyhow::Context;
    pub use bevy::prelude::*;
}

// generic undo for entity add. Can be re-used by all commands that just add an entity that can be removed by adding components::Despawn.
// NOTE: make sure that there is a system that handles components::Despawn, since the actual despawn may need a specific implementation.
pub mod add_entity {

    use super::prelude::*;
    pub struct Undo {
        pub entity: Entity,
    }
    impl UndoCommand for Undo {
        fn try_merge(&mut self, _other: &dyn UndoCommand) -> bool {
            false
        }

        fn undo(&self, undo_commands: &mut UndoCommands) -> Result<()> {
            let entity = undo_commands.undo_stack.remap_entity(self.entity);
            if let Some(mut entity_commands) = undo_commands.commands.get_entity(entity) {
                entity_commands.insert(components::Despawn);
                Ok(())
            } else {
                error!( "failed to despawn {:?} to undo addition. Either missing remap or undo failed earlier", entity);
                Err(EditCommandError::UnknownEntity(entity).into())
            }
        }
    }
}

#[derive(SystemParam)]
pub struct EditCommands<'w, 's> {
    commands: Commands<'w, 's>,
    undo_stack: ResMut<'w, UndoStack>,
    pub brush_query: Query<
        'w,
        's,
        (
            &'static mut components::BrushMaterialProperties,
            &'static csg::Brush,
        ),
    >,
    pub transform_query: Query<'w, 's, &'static mut Transform, With<components::EditablePoint>>,
}
impl<'w, 's> EditCommands<'w, 's> {
    pub fn apply(&mut self, cmd: impl EditCommand) -> Result<()> {
        let undo_cmd = cmd.apply(self)?;
        self.undo_stack.push_generic(undo_cmd);
        Ok(())
    }

    pub fn end_drag(&mut self, entity: Entity) {
        self.commands
            .entity(entity)
            .remove::<components::DragAction>();

        self.undo_stack.commit();
    }
}
