use super::prelude::*;

pub struct Command {
    pub entity: Entity,
    pub start_brush: csg::Brush,
    pub start_material_props: components::BrushMaterialProperties,
    pub brush: csg::Brush,
    pub material_props: components::BrushMaterialProperties,
}

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>> {
        let mut entity_commands = commands
            .commands
            .get_entity(self.entity)
            .ok_or(EditCommandError::UnknownEntity(self.entity))
            .context("apply brush_clip")?;

        let (mut material_props, _) = commands
            .brush_query
            .get_mut(self.entity)
            .context("apply brush_clip")?;

        *material_props = self.material_props.clone();
        entity_commands.insert(components::EditUpdate::BrushDrag {
            brush: self.brush.clone(),
        });

        Ok(Box::new(self))
    }
}

impl UndoCommand for Command {
    fn try_merge(&mut self, other: &dyn UndoCommand) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            if self.entity == other.entity {
                self.brush = other.brush.clone();
                info!("merged");
                return true;
            }
        }
        false
    }
    fn undo(&self, undo_commands: &mut UndoCommands) -> Result<()> {
        let entity = undo_commands.undo_stack.remap_entity(self.entity);
        let mut entity_commands = undo_commands
            .commands
            .get_entity(entity)
            .ok_or(EditCommandError::UnknownEntity(entity))
            .context("undo brush_clip")?;

        let mut material_props = undo_commands
            .material_properties_query
            .get_mut(self.entity)
            .context("undo brush_clip")?;

        *material_props = self.start_material_props.clone();

        entity_commands.insert(components::EditUpdate::BrushDrag {
            brush: self.start_brush.clone(),
        });
        Ok(())
    }
}
