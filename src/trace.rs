use bevy::{math::Vec3, prelude::*};
use bevy_rapier3d::prelude::*;

#[derive(Debug, Clone)]
pub struct TraceContact {
    pub collider_normal: Vec3,
    pub collider_point: Vec3,
    pub shape_normal: Vec3,
    pub shape_point: Vec3,
}

pub struct TraceResult {
    pub contact: Option<TraceContact>,
    pub stuck: bool,
    pub dist: Vec3,
    pub f: f32,
}

#[derive(Debug, Clone)]
pub enum CastResult {
    NoHit,
    Impact(f32, TraceContact),
    // Touch(Contact),
    Stuck,
    Failed,
}

pub trait CollisionTraceable {
    fn trace2(&self, start: Vec3, dist: Vec3) -> TraceResult;
}

impl CollisionTraceable for RapierContext {
    fn trace2(&self, start: Vec3, dist: Vec3) -> TraceResult {
        let shape = Collider::cylinder(0.9, 0.2);
        // let shape = Cuboid::new(Vec3::new(0.2, 0.9, 0.2).into());

        let shape_pos = start;
        let shape_rot = Quat::default();
        let shape_vel = dist;
        let filter = QueryFilter::default();

        let d = dist.length();
        const MIN_DIST: f32 = 1e-2;

        if d <= MIN_DIST {
            return TraceResult {
                contact: None,
                dist: Vec3::ZERO,
                stuck: false,
                f: 1.0,
            };
        }

        let minfrac = MIN_DIST / d;

        // info!("minfrac: {:?} {} {}", dist, d, minfrac);

        let trace_result = if let Some((_handle, toi)) = self.cast_shape(
            shape_pos,
            shape_rot,
            shape_vel,
            &shape,
            ShapeCastOptions::with_max_time_of_impact(1.0),
            filter,
        ) {
            if let Some(hit) = toi.details {
                let contact = TraceContact {
                    collider_normal: (hit.normal1),
                    collider_point: hit.witness1,
                    shape_normal: hit.normal2,
                    shape_point: hit.witness2,
                };

                match toi.status {
                    ShapeCastStatus::Converged if toi.time_of_impact > minfrac => TraceResult {
                        contact: Some(contact),
                        dist: dist * (toi.time_of_impact - minfrac),
                        stuck: false,
                        f: toi.time_of_impact - minfrac,
                    },
                    ShapeCastStatus::Converged
                    | ShapeCastStatus::Failed
                    | ShapeCastStatus::OutOfIterations => TraceResult {
                        contact: Some(contact),
                        dist: Vec3::ZERO,
                        stuck: false,
                        f: 0.0,
                    },
                    ShapeCastStatus::PenetratingOrWithinTargetDist => TraceResult {
                        contact: None,
                        dist: Vec3::ZERO,
                        stuck: true,
                        f: 0.0,
                    },
                }
            } else {
                TraceResult {
                    contact: None,
                    dist,
                    stuck: false,
                    f: 1.0,
                }
            }
        } else {
            TraceResult {
                contact: None,
                dist,
                stuck: false,
                f: 1.0,
            }
        };

        let end_pos = start + trace_result.dist;
        let intersection = self.intersection_with_shape(end_pos, Quat::default(), &shape, filter);
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

    // fn trace2(&self, start: Vec3, dist: Vec3) -> TraceResult {
    //     let shape = Collider::cylinder(0.9, 2.0);
    //     let shape_pos = start;
    //     let shape_rot = Quat::default();
    //     let shape_vel = dist;
    //     let max_toi = 1.0;
    //     let filter = QueryFilter::default();

    //     if let Some((entity, hit)) =
    //         self.cast_shape(shape_pos, shape_rot, shape_vel, &shape, max_toi, filter)
    //     {
    //         // The first collider hit has the entity `entity`. The `hit` is a
    //         // structure containing details about the hit configuration.
    //         println!(
    //             "Hit the entity {:?} with the configuration: {:?}",
    //             entity, hit
    //         );

    //         let contact = TraceContact {
    //             collider_normal: (hit.normal1),
    //             collider_point: hit.witness1,
    //             shape_normal: (hit.normal2),
    //             shape_point: hit.witness2,
    //         };
    //         const MIN_DIST: f32 = 1e-2;
    //         let d = dist.length();
    //         let minfrac = MIN_DIST / d;

    //         match hit.status {
    //             TOIStatus::Converged if hit.toi > minfrac => TraceResult {
    //                 contact: Some(contact),
    //                 dist: dist * (hit.toi - minfrac),
    //                 stuck: false,
    //                 f: hit.toi - minfrac,
    //             },
    //             TOIStatus::Converged | TOIStatus::Failed | TOIStatus::OutOfIterations => {
    //                 TraceResult {
    //                     contact: Some(contact),
    //                     dist: Vec3::ZERO,
    //                     stuck: false,
    //                     f: 0.0,
    //                 }
    //             }
    //             TOIStatus::Penetrating => TraceResult {
    //                 contact: None,
    //                 dist: Vec3::ZERO,
    //                 stuck: true,
    //                 f: 0.0,
    //             },
    //         }
    //     } else {
    //         TraceResult {
    //             contact: None,
    //             dist: dist,
    //             stuck: false,
    //             f: 1.0,
    //         }
    //     }

    //     // TraceResult {
    //     //     contact: None,
    //     //     dist,
    //     //     stuck: false,
    //     //     f: 1.0,
    //     // }
    // }
}

// pub struct CollisionSystem<'a, 'x, 'world, 'state> {
//     pub contact_debug: &'a mut ContactDebug,
//     pub query_pipeline: &'a QueryPipeline,
//     pub collider_query: &'a QueryPipelineColliderComponentsQuery<'world, 'state, 'x>,
// }

// impl<'a, 'x, 'world, 'state> CollisionTraceable for CollisionSystem<'a, 'x, 'world, 'state> {
//     fn trace2(&self, start: Vec3, dist: Vec3) -> TraceResult {
//         let collider_set = QueryPipelineColliderComponentsSet(self.collider_query);
//         let shape = Cylinder::new(0.9, 0.2);
//         // let shape = Cuboid::new(Vec3::new(0.2, 0.9, 0.2).into());

//         let shape_pos = Isometry::translation(start.x, start.y, start.z);
//         let shape_vel = dist.into();
//         let groups = InteractionGroups::all();
//         let filter = None;

//         let d = dist.length();collider_query
//         const MIN_DIST: f32 = 1e-2;

//         if d <= MIN_DIST {
//             return TraceResult {
//                 contact: None,
//                 dist: Vec3::ZERO,
//                 stuck: false,
//                 f: 1.0,
//             };
//         }

//         let minfrac = MIN_DIST / d;

//         // info!("minfrac: {:?} {} {}", dist, d, minfrac);

//         let trace_result = if let Some((_handle, hit)) = self.query_pipeline.cast_shape(
//             &collider_set,
//             &shape_pos,
//             &shape_vel,
//             &shape,
//             1.0,
//             groups,
//             filter,
//         ) {
//             use bevy_rapier3d::rapier::parry::query::TOIStatus;

//             let contact = TraceContact {
//                 collider_normal: (*hit.normal1).into(),
//                 collider_point: hit.witness1.into(),
//                 shape_normal: (*hit.normal2).into(),
//                 shape_point: hit.witness2.into(),
//             };

//             match hit.status {
//                 TOIStatus::Converged if hit.toi > minfrac => TraceResult {
//                     contact: Some(contact),
//                     dist: dist * (hit.toi - minfrac),
//                     stuck: false,
//                     f: hit.toi - minfrac,
//                 },
//                 TOIStatus::Converged | TOIStatus::Failed | TOIStatus::OutOfIterations => {
//                     TraceResult {
//                         contact: Some(contact),
//                         dist: Vec3::ZERO,
//                         stuck: false,
//                         f: 0.0,
//                     }
//                 }
//                 TOIStatus::Penetrating => TraceResult {
//                     contact: None,
//                     dist: Vec3::ZERO,
//                     stuck: true,
//                     f: 0.0,
//                 },
//             }
//         } else {
//             TraceResult {
//                 contact: None,
//                 dist,
//                 stuck: false,
//                 f: 1.0,
//             }
//         };

//         let end_pos = start + trace_result.dist;
//         let intersection = self.query_pipeline.intersection_with_shape(
//             &collider_set,
//             &end_pos.into(),
//             &shape,
//             groups,
//             filter,
//         );
//         if let Some(_collider) = intersection {
//             warn!(
//                 "trace ends up stuck. {:?} {} {} {}",
//                 trace_result.dist,
//                 trace_result.f,
//                 dist,
//                 dist.length()
//             );
//             return TraceResult {
//                 contact: None,
//                 dist: Vec3::ZERO,
//                 stuck: false,
//                 f: 0.0,
//             };
//         }

//         trace_result
//     }
// }
