use std::any::Any;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::csg;

use super::{
    components,
    undo::{self, UndoCommands, UndoStack},
};

pub trait UndoDowncast {
    fn as_any(&self) -> &dyn Any;
}

pub trait UndoCommand: UndoDowncast {
    fn try_merge(&mut self, other: &dyn UndoCommand) -> bool;
    fn undo(&self, undo_commands: &mut UndoCommands);
}

impl<T: Any> UndoDowncast for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait EditCommand {
    fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync>;
}

pub mod prelude {
    pub use super::{EditCommand, EditCommands, UndoCommand};
    pub use crate::{
        csg,
        editor::{components, undo::UndoCommands},
    };
    pub use bevy::prelude::*;
}

pub mod update_brush_drag {
    use super::prelude::*;

    pub struct Command {
        pub entity: Entity,
        pub start_brush: csg::Brush,
        pub brush: csg::Brush,
    }

    impl EditCommand for Command {
        fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
            commands
                .commands
                .entity(self.entity)
                .insert(components::EditUpdate::BrushDrag {
                    brush: self.brush.clone(),
                });
            Box::new(self)
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
        fn undo(&self, undo_commands: &mut UndoCommands) {
            let entity = undo_commands.undo_stack.remap_entity(self.entity);
            undo_commands
                .commands
                .entity(entity)
                .insert(components::EditUpdate::BrushDrag {
                    brush: self.start_brush.clone(),
                });
        }
    }
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

        fn undo(&self, undo_commands: &mut UndoCommands) {
            let entity = undo_commands.undo_stack.remap_entity(self.entity);
            undo_commands
                .commands
                .entity(entity)
                .insert(components::Despawn);
        }
    }
}

pub mod add_brush {
    use super::prelude::*;

    pub struct Command {
        pub brush: csg::Brush,
    }
    pub use super::add_entity::Undo;

    impl EditCommand for Command {
        fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
            let entity = commands
                .commands
                .spawn((
                    components::EditorObjectBrushBundle::from_brush(self.brush),
                    components::Selected,
                ))
                .id();

            Box::new(Undo { entity })
        }
    }
}

pub mod duplicate_brush {
    use super::prelude::*;

    pub struct Command {
        pub template_entity: Entity,
    }

    pub use super::add_entity::Undo;

    impl EditCommand for Command {
        fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
            if let Ok((material_properties, brush)) = commands.brush_query.get(self.template_entity)
            {
                let entity = commands
                    .commands
                    .spawn((
                        components::EditorObjectBrushBundle::from_brush(brush.clone())
                            .with_material_properties(material_properties.clone()),
                        components::Selected,
                    ))
                    .id();

                return Box::new(Undo { entity });
            }

            // FIXME: make apply fallible (if that makes sense)
            panic!("could not find template brush {:?}", self.template_entity);
        }
    }
}

pub mod add_pointlight {
    use super::prelude::*;

    pub struct Command;
    pub use super::add_entity::Undo;
    impl EditCommand for Command {
        fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
            let entity = commands
                .commands
                .spawn((
                    components::EditorObjectPointlightBundle::default(),
                    components::Selected,
                ))
                .id();

            Box::new(Undo { entity })
        }
    }
}

pub mod set_brush_material {

    use super::prelude::*;

    pub struct Command {
        pub entity: Entity,
        pub face: i32,
        pub material: String,
    }

    pub struct Undo {
        pub entity: Entity,
        pub face: i32,
        pub old_material: String,
    }

    impl EditCommand for Command {
        fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
            if let Ok((mut material_props, _)) = commands.brush_query.get_mut(self.entity) {
                let old_material = std::mem::replace(
                    &mut material_props.materials[self.face as usize],
                    self.material,
                );
                commands
                    .commands
                    .entity(self.entity)
                    .insert(components::CsgDirty);

                return Box::new(Undo {
                    entity: self.entity,
                    face: self.face,
                    old_material,
                });
            }

            panic!("material props not found for {:?}", self.entity);
        }
    }

    impl UndoCommand for Undo {
        fn try_merge(&mut self, other: &dyn UndoCommand) -> bool {
            false
        }

        fn undo(&self, undo_commands: &mut UndoCommands) {
            let entity = undo_commands.undo_stack.remap_entity(self.entity);

            if let Ok(mut material_props) = undo_commands.material_properties_query.get_mut(entity)
            {
                material_props.materials[self.face as usize] = self.old_material.clone();
                undo_commands
                    .commands
                    .entity(entity)
                    .insert(components::CsgDirty);
            }
        }
    }
}

pub mod remove_entity {
    use crate::editor::components::BrushMaterialProperties;

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
                Undo::NotImplemented => warn!("undo not implemented for entity remove"),
            }
        }
    }
}

pub mod update_point_transform {
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
    pub fn apply(&mut self, cmd: impl EditCommand) {
        let undo_cmd = cmd.apply(self);
        self.undo_stack.push_generic(undo_cmd);
    }

    pub fn end_drag(&mut self, entity: Entity) {
        self.commands
            .entity(entity)
            .remove::<components::DragAction>();

        self.undo_stack.commit();
    }
}
