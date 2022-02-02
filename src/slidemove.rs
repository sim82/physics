use std::{collections::VecDeque, time::Duration};

use bevy::{input::mouse::MouseMotion, math::Vec3, prelude::*, render::mesh};
// use bevy_rapier3d::physics::{
//     QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
// };
use bevy_rapier3d::prelude::*;

use crate::{
    contact_debug::ContactDebug,
    trace::{self, CastResult},
};

fn slidemove_none(
    contact_debug: &mut ContactDebug,
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    velocity: Vec3,
    time: f32,
    query_pipeline: &Res<QueryPipeline>,
) -> Vec3 {
    let res = trace::trace2(collider_query, origin, velocity * time, query_pipeline);

    if res.stuck {
        info!("stuck!");
        return Vec3::ZERO;
    }

    res.dist
    // match &res {
    //     CastResult::Impact(toi, ref contact) => contact_debug
    //         .add
    //         .push((contact.clone(), origin + velocity * *toi)),
    //     // CastResult::Touch(ref contact) => contact_debug.add.push((contact.clone(), origin)),
    //     _ => (),
    // }

    // match res {
    //     CastResult::NoHit => velocity * time,
    //     CastResult::Impact(toi, _) => {
    //         info!("impact: {}", toi);
    //         velocity * toi
    //     }
    //     CastResult::Stuck => {
    //         info!("stuck!");
    //         Vec3::ZERO
    //     }
    //     CastResult::Failed => Vec3::ZERO,
    // }
}

// port of quake 3 PM_ClipVelocity
fn do_clip_velocity(v_in: Vec3, normal: Vec3, overbounce: f32) -> Vec3 {
    let backoff = match v_in.dot(normal) {
        dot if dot < 0.0 => dot * overbounce,
        dot => dot / overbounce,
    };
    let change = Vec3::new(normal.x * backoff, normal.y * backoff, normal.z * backoff);
    v_in - change
}

fn step_slidemove_try1(
    contact_debug: &mut ContactDebug,
    // debug_lines: &mut debug_lines::DebugLines,
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    mut velocity: Vec3,
    mut time: f32,
    query_pipeline: &Res<QueryPipeline>,
) -> Vec3 {
    let (mut move_v, bump) = slidemove_try1(
        contact_debug,
        collider_query,
        origin,
        velocity,
        time,
        query_pipeline,
    );
    // info!("bump: {:?}", bump);
    if !bump {
        // goal reached without wall interaction -> done
        return move_v;
    }
    const STEP_SIZE: f32 = 0.11;
    let step_dir = -Vec3::Y;
    // groundtrace
    let res = trace::trace(collider_query, origin, step_dir, query_pipeline, STEP_SIZE);
    info!("ground trace1: {:?}", res);
    let toi = match res {
        CastResult::NoHit | CastResult::Stuck | CastResult::Failed => {
            return move_v - step_dir * STEP_SIZE
        }
        CastResult::Impact(toi, _) => toi,
        // CastResult::Touch(_) => 0.0,
    };

    let mut move_v = step_dir * toi; // * 0.99;

    // hop
    let step_dir = Vec3::Y;

    let res = trace::trace(
        collider_query,
        origin + move_v,
        step_dir,
        query_pipeline,
        STEP_SIZE,
    );

    info!("hop: {:?}", res);

    let toi = match res {
        CastResult::Stuck | CastResult::Failed => return move_v,
        CastResult::NoHit => STEP_SIZE,
        CastResult::Impact(toi, _) => toi,
        // CastResult::Touch(_) => 0.0,
    };

    move_v += step_dir * toi;

    let (move_v2, bump) = slidemove_try1(
        contact_debug,
        collider_query,
        origin + move_v,
        velocity,
        time,
        query_pipeline,
    );
    move_v += move_v2;

    // groundtrace
    let step_dir = -Vec3::Y;

    let res = trace::trace(
        collider_query,
        origin + move_v,
        step_dir,
        query_pipeline,
        STEP_SIZE,
    );
    info!("ground trace2: {:?}", res);

    let toi = match res {
        CastResult::NoHit | CastResult::Stuck | CastResult::Failed => {
            return move_v - step_dir * STEP_SIZE
        }
        CastResult::Impact(toi, _) => toi,
        // CastResult::Touch(_) => 0.0,
    };

    move_v += step_dir * toi * 0.99;

    move_v
}
pub fn slidemove_try1(
    contact_debug: &mut ContactDebug,
    // debug_lines: &mut debug_lines::DebugLines,
    collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    mut velocity: Vec3,
    mut time: f32,
    query_pipeline: &Res<QueryPipeline>,
) -> (Vec3, bool) {
    let mut planes = Vec::new();
    let mut move_v = Vec3::ZERO;

    // TODO: gravity and ground trace
    // info!("slidemove");
    // initial velocity defines first clipping plane -> avoid to be nudged backwards (due to overclip?)
    planes.push(velocity.normalize());
    planes.push(Vec3::Y);

    info!("start");
    'bump: for bump in 0..4 {
        let trace_start_pos = origin + move_v;
        let trace_dist = velocity * time;

        let res = trace::trace2(collider_query, trace_start_pos, trace_dist, query_pipeline);
        if res.stuck {
            error!("stuck!");
            return (Vec3::ZERO, true);
        }
        // info!(
        //     "bump {} {:?} {} {:?} {}",
        //     bump, trace_dist, res.f, velocity, time
        // );

        if let Some(contact) = res.contact {
            contact_debug
                .add
                .push((contact.clone(), trace_start_pos + res.dist));

            // use contact normal as clip plane
            move_v += res.dist;
            time -= time * res.f;

            //
            // if this is the same plane we hit before, nudge velocity
            // out along it, which fixes some epsilon issues with
            // non-axial planes
            //
            for plane in planes.iter() {
                let dot = contact.collider_normal.dot(*plane);
                if dot > 0.99 {
                    info!("dot: {} {:?} {:?}", dot, contact.collider_normal, plane);

                    velocity += contact.collider_normal;
                    info!("nudge");
                    continue 'bump;
                }
                info!("dot: {} {:?} {:?}", dot, contact.collider_normal, plane);
            }
            planes.push(contact.collider_normal);
        } else {
            info!("done");
            return (move_v + res.dist, false);
        }

        // if bump >= 1 {
        //     info!("bump: {}", bump);
        // }

        // actual clipping: try to make velocity parallel to all clip planes
        // find first plane we intersect
        for (i, plane) in planes.iter().enumerate() {
            let into = velocity.normalize().dot(*plane);
            if into >= 0.1 {
                continue; // move doesn't interact with the plane
            }
            // TODO: store impact speed

            // slide along the plane
            const OVERCLIP: f32 = 1.001;
            let mut clip_velocity = do_clip_velocity(velocity, *plane, OVERCLIP);
            // TODO: end velocity for gravity
            info!(
                "clip velocity1 {:?} {:?} {:?} {} {}",
                velocity, clip_velocity, plane, i, into
            );
            // find second plane
            for (j, plane2) in planes.iter().enumerate() {
                if j == i {
                    continue;
                }

                if clip_velocity.dot(*plane2) >= 0.1 {
                    continue;
                }

                // re-clip velocity with second plane
                let xc = clip_velocity;
                clip_velocity = do_clip_velocity(clip_velocity, *plane2, OVERCLIP);
                if clip_velocity.dot(*plane) >= 0.0 {
                    info!(
                        "clip velocity1.5 {:?} {:?} {:?} {}",
                        xc, clip_velocity, plane2, j
                    );

                    continue;
                }
                // info!(
                //     "clip velocity2 {:?} {:?} {:?} {}",
                //     xc, clip_velocity, plane2, j
                // );

                // slide along the crease of the two planes (based on original velocity!)
                let dir = plane.cross(*plane2).normalize();

                let d = dir.dot(velocity);
                clip_velocity = dir * d;

                // is there a third plane we clip?
                for (k, plane3) in planes.iter().enumerate() {
                    if k == i || k == j {
                        continue;
                    }
                    if clip_velocity.dot(*plane3) >= 0.1 {
                        continue;
                    }

                    // give up on triple plane intersections
                    warn!(
                        "triple plane interaction {:?} {:?} {:?}",
                        plane, plane2, plane3
                    );
                    return (Vec3::ZERO, false);
                }
            }
            // all interactions should be fixed -> try another move
            velocity = clip_velocity;
            break;
        }
    }
    (move_v, true)
}
