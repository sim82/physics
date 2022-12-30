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
    fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
        if let Ok(mut transform) = commands.transform_query.get_mut(self.entity) {
            let old_transform = *transform;
            transform.translation = self.transform.translation;

            return Box::new(Undo {
                entity: self.entity,
                old_transform,
            });
        }

        panic!("transform not found for {:?}", self.entity);
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

    fn undo(&self, undo_commands: &mut UndoCommands) {
        if let Ok(mut transform) = undo_commands.transform_query.get_mut(self.entity) {
            *transform = self.old_transform;
        } else {
            warn!("failed to undo point transform on {:?}", self.entity);
        }
    }
}