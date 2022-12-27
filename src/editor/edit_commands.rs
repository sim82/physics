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
    pub brush_query: Query<
        'w,
        's,
        (
            &'static mut components::BrushMaterialProperties,
            &'static csg::Brush,
        ),
    >,
}
impl<'w, 's> EditCommands<'w, 's> {
    pub fn update_brush_drag(
        &mut self,
        entity: Entity,
        start_brush: &csg::Brush,
        brush: csg::Brush,
    ) {
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

    pub fn add_brush(&mut self, brush: csg::Brush) {
        let entity = self
            .commands
            .spawn((
                components::EditorObjectBrushBundle::from_brush(brush),
                components::Selected,
            ))
            .id();

        self.undo_stack.push_entity_add(entity);
        // info!("new brush: {:?}", entity);
    }

    pub fn duplicate_brush(&mut self, entity: Entity) {
        if let Ok((material_properties, brush)) = self.brush_query.get(entity) {
            self.commands.spawn((
                components::EditorObjectBrushBundle::from_brush(brush.clone())
                    .with_material_properties(material_properties.clone()),
                components::Selected,
            ));
        }
    }

    pub fn set_brush_material(&mut self, entity: Entity, face: i32, material: String) {
        if let Ok((mut material_props, _)) = self.brush_query.get_mut(entity) {
            let old_material = std::mem::replace(
                &mut material_props.materials[face as usize],
                material.clone(),
            );

            self.undo_stack
                .push_matrial_set(entity, face, old_material, material);
        }
    }
}
