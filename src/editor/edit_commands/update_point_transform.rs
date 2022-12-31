use super::prelude::*;

pub struct Command {
    pub entity: Entity,
    pub transform: Transform,
}

pub struct Undo {
    pub entity: Entity,
    pub old_transform: Transform,
}

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>> {
        let mut transform = commands
            .transform_query
            .get_mut(self.entity)
            .context("apply update_point_transform")?;
        let old_transform = *transform;
        transform.translation = self.transform.translation;

        Ok(Box::new(Undo {
            entity: self.entity,
            old_transform,
        }))

        // panic!("transform not found for {:?}", self.entity);
    }
}

impl UndoCommand for Undo {
    fn try_merge(&mut self, other: &dyn UndoCommand) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            if self.entity == other.entity {
                info!("merged");
                return true;
            }
        }
        false
    }

    fn undo(&self, undo_commands: &mut UndoCommands) -> Result<()> {
        let mut transform = undo_commands
            .transform_query
            .get_mut(self.entity)
            .context("undo update_point_transform")?;
        *transform = self.old_transform;
        Ok(())
        // warn!("failed to undo point transform on {:?}", self.entity);
        // }
    }
}
