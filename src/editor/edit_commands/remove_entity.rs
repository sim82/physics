use crate::editor::components::{BrushMaterialProperties, PointLightProperties};

use super::prelude::*;

pub struct Command {
    pub entity: Entity,
}

pub enum Undo {
    Brush {
        entity: Entity,
        brush: csg::Brush,
        material_props: BrushMaterialProperties,
    },
    NotImplemented,
}

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
        let undo = if let Ok((material_props, brush)) = commands.brush_query.get(self.entity) {
            Box::new(Undo::Brush {
                entity: self.entity,
                brush: brush.clone(),
                material_props: material_props.clone(),
            })
        } else {
            Box::new(Undo::NotImplemented)
        };
        commands
            .commands
            .entity(self.entity)
            .insert(components::Despawn);
        undo
    }
}

impl UndoCommand for Undo {
    fn try_merge(&mut self, _other: &dyn UndoCommand) -> bool {
        false
    }

    fn undo(&self, undo_commands: &mut UndoCommands) {
        match self {
            Undo::Brush {
                entity,
                brush,
                material_props,
            } => {
                let new_entity = undo_commands
                    .commands
                    .spawn(
                        components::EditorObjectBrushBundle::from_brush(brush.clone())
                            .with_material_properties(material_props.clone()),
                    )
                    .id();
                undo_commands
                    .undo_stack
                    .entity_recreate_map
                    .insert(*entity, new_entity);
            }
            Undo::NotImplemented => warn!("undo not implemented for add entity."),
        }
    }
}
