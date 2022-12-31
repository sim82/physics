use super::prelude::*;

pub struct Command {
    pub entity: Entity,
    pub start_brush: csg::Brush,
    pub brush: csg::Brush,
}

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>> {
        commands
            .commands
            .get_entity(self.entity)
            .ok_or(EditCommandError::UnknownEntity(self.entity))?
            .insert(components::EditUpdate::BrushDrag {
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
        undo_commands
            .commands
            .get_entity(entity)
            .ok_or(EditCommandError::UnknownEntity(entity))?
            .insert(components::EditUpdate::BrushDrag {
                brush: self.start_brush.clone(),
            });
        Ok(())
    }
}
