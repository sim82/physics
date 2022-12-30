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

    // pub fn add_brush(&mut self, brush: csg::Brush) {
    //     // let entity = self
    //     //     .commands
    //     //     .spawn((
    //     //         components::EditorObjectBrushBundle::from_brush(brush),
    //     //         components::Selected,
    //     //     ))
    //     //     .id();

    //     // self.undo_stack.push_entity_add(entity);
    //     // info!("new brush: {:?}", entity);
    // }

    pub fn add_pointlight(&mut self) {
        let entity = self
            .commands
            .spawn((
                components::EditorObjectPointlightBundle::default(),
                components::Selected,
            ))
            .id();

        self.undo_stack.push_entity_add(entity);
    }
    // pub fn duplicate_brush(&mut self, template_entity: Entity) {
    // if let Ok((material_properties, brush)) = self.brush_query.get(template_entity) {
    //     let entity = self
    //         .commands
    //         .spawn((
    //             components::EditorObjectBrushBundle::from_brush(brush.clone())
    //                 .with_material_properties(material_properties.clone()),
    //             components::Selected,
    //         ))
    //         .id();
    //     self.undo_stack.push_entity_add(entity);
    // }
    // }

    pub fn set_brush_material(&mut self, entity: Entity, face: i32, material: String) {
        if let Ok((mut material_props, _)) = self.brush_query.get_mut(entity) {
            let old_material = std::mem::replace(
                &mut material_props.materials[face as usize],
                material.clone(),
            );
            self.commands.entity(entity).insert(components::CsgDirty);

            self.undo_stack
                .push_matrial_set(entity, face, old_material, material);
        }
    }

    pub fn remove_entity(&mut self, primary: Entity) {
        if let Ok((material_props, brush)) = self.brush_query.get(primary) {
            self.undo_stack
                .push_brush_remove(primary, brush.clone(), material_props.clone());
        }
        self.commands.entity(primary).insert(components::Despawn);
    }

    pub fn update_point_transform(&mut self, entity: Entity, update: Transform) {
        if let Ok(mut transform) = self.transform_query.get_mut(entity) {
            let old_transform = *transform;
            transform.translation = update.translation;

            self.undo_stack
                .push_point_drag(entity, old_transform, *transform);
        }
    }
}
