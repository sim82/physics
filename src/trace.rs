use bevy::{input::mouse::MouseMotion, math::Vec3, prelude::*, render::mesh};
use bevy_rapier3d::prelude::*;
use std::{collections::VecDeque, time::Duration};

#[derive(Debug, Clone)]
pub struct Contact {
    pub collider_normal: Vec3,
    pub collider_point: Vec3,
    pub shape_normal: Vec3,
    pub shape_point: Vec3,
}

pub struct TraceResult {
    pub contact: Option<Contact>,
    pub stuck: bool,
    pub dist: Vec3,
    pub f: f32,
}

#[derive(Debug, Clone)]
pub enum CastResult {
    NoHit,
    Impact(f32, Contact),
    // Touch(Contact),
    Stuck,
    Failed,
}

pub fn trace(
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    velocity: Vec3,
    query_pipeline: &Res<QueryPipeline>,
    time: f32,
) -> CastResult {
    let collider_set = QueryPipelineColliderComponentsSet(collider_query);
    let shape = Cylinder::new(0.9, 0.2);
    let shape_pos = Isometry::translation(origin.x, origin.y, origin.z);
    let shape_vel = velocity.into();
    let groups = InteractionGroups::all();
    let filter = None;
    if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        time,
        groups,
        filter,
    ) {
        use bevy_rapier3d::rapier::parry::query::TOIStatus;

        let contact = Contact {
            collider_normal: (*hit.normal1).into(),
            collider_point: hit.witness1.into(),
            shape_normal: (*hit.normal2).into(),
            shape_point: hit.witness2.into(),
        };
        match hit.status {
            TOIStatus::Converged if hit.toi > 0.01 => CastResult::Impact(hit.toi * 0.99, contact),
            TOIStatus::Converged => CastResult::Impact(0.0, contact),
            // TOIStatus::Converged => CastResult::Touch(contact),
            TOIStatus::Failed | TOIStatus::OutOfIterations => CastResult::Failed,
            TOIStatus::Penetrating => CastResult::Stuck,
        }
    } else {
        CastResult::NoHit
    }
}

pub fn trace2(
    collider_query: &QueryPipelineColliderComponentsQuery,
    start: Vec3,
    dist: Vec3,
    query_pipeline: &Res<QueryPipeline>,
) -> TraceResult {
    let collider_set = QueryPipelineColliderComponentsSet(collider_query);
    let shape = Cylinder::new(0.9, 0.2);
    // let shape = Cuboid::new(Vec3::new(0.2, 0.9, 0.2).into());

    let shape_pos = Isometry::translation(start.x, start.y, start.z);
    let shape_vel = dist.into();
    let groups = InteractionGroups::all();
    let filter = None;

    let d = dist.length();
    const MIN_DIST: f32 = 1e-2;

    if d <= MIN_DIST {
        return TraceResult {
            contact: None,
            dist: Vec3::ZERO,
            stuck: false,
            f: 0.0,
        };
    }

    let minfrac = MIN_DIST / d;

    info!("minfrac: {:?} {} {}", dist, d, minfrac);

    let trace_result = if let Some((handle, hit)) = query_pipeline.cast_shape(
        &collider_set,
        &shape_pos,
        &shape_vel,
        &shape,
        1.0,
        groups,
        filter,
    ) {
        use bevy_rapier3d::rapier::parry::query::TOIStatus;

        let contact = Contact {
            collider_normal: (*hit.normal1).into(),
            collider_point: hit.witness1.into(),
            shape_normal: (*hit.normal2).into(),
            shape_point: hit.witness2.into(),
        };

        match hit.status {
            TOIStatus::Converged if hit.toi > minfrac => TraceResult {
                contact: Some(contact),
                dist: dist * (hit.toi - minfrac),
                stuck: false,
                f: hit.toi - minfrac,
            },
            TOIStatus::Converged | TOIStatus::Failed | TOIStatus::OutOfIterations => TraceResult {
                contact: Some(contact),
                dist: Vec3::ZERO,
                stuck: false,
                f: 0.0,
            },
            TOIStatus::Penetrating => TraceResult {
                contact: None,
                dist: Vec3::ZERO,
                stuck: true,
                f: 0.0,
            },
        }
    } else {
        TraceResult {
            contact: None,
            dist: dist * (1.0 - minfrac),
            stuck: false,
            f: 1.0 - minfrac,
        }
    };

    let end_pos = start + trace_result.dist;
    let intersection = query_pipeline.intersection_with_shape(
        &collider_set,
        &end_pos.into(),
        &shape,
        groups,
        filter,
    );
    if let Some(_collider) = intersection {
        warn!(
            "trace ends up stuck. {:?} {} {} {}",
            trace_result.dist,
            trace_result.f,
            dist,
            dist.length()
        );
        return TraceResult {
            contact: None,
            dist: Vec3::ZERO,
            stuck: false,
            f: 0.0,
        };
    }

    trace_result
}
