use bevy::{ecs::system::SystemParam, prelude::*};

use crate::csg;

use super::{
    components,
    undo::{self, UndoStack},
};

#[derive(SystemParam)]
pub struct EditCommands<'w, 's> {
    commands: Commands<'w, 's>,
    undo_stack: ResMut<'w, UndoStack>,
}
impl<'w, 's> EditCommands<'w, 's> {
    pub fn brush_drag(&mut self, entity: Entity, start_brush: &csg::Brush, brush: csg::Brush) {
        self.undo_stack.push_brush_drag(entity, start_brush, &brush);
        self.commands
            .entity(entity)
            .insert(components::EditUpdate::BrushDrag { brush });
    }

    pub fn end_brush_drag(&mut self, entity: Entity) {
        self.commands
            .entity(entity)
            .remove::<components::DragAction>();

        if !matches!(self.undo_stack.stack.last(), Some(undo::UndoEntry::BrushDrag { entity: top_entity, start_brush: _, brush: _ }) if *top_entity == entity)
        {
            warn!("undo stack top entity doesn ot match end_brush_drag")
        }
        self.undo_stack.commit();
    }
}
