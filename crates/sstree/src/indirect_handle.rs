use std::collections::HashMap;
use std::marker::PhantomData;

use arrayvec::ArrayVec;
use bevy::prelude::{Resource, Vec3};

pub const MAX_TREE_HEIGHT: usize = 16;

pub trait Distance {
    fn distance(&self, other: &Self) -> f32;
}

pub trait DimIndex:
    std::ops::Index<usize, Output = f32> + std::ops::IndexMut<usize, Output = f32>
{
    const NUM_DIMENSIONS: usize;
}

pub trait Center: Distance + Default + DimIndex + PartialEq {}
impl<K: Distance + Default + DimIndex + PartialEq> Center for K {}

pub trait Radius: PartialEq {}
impl<R: PartialEq> Radius for R {}

#[derive(Debug, Default)]
pub struct Bounds<K: Center> {
    pub center: K,
    pub radius: f32,
}

impl<K: Center> Bounds<K> {
    fn from_center_radius(center: K, radius: f32) -> Self {
        Self { center, radius }
    }
    fn distance_point(&self, p2: &K) -> f32 {
        self.center.distance(p2)
    }
    fn distance(&self, p2: &Self) -> f32 {
        self.center.distance(&p2.center)
    }
    fn intersects_point(&self, target: &K) -> bool {
        self.center.distance(target) <= self.radius
    }
    fn intersects(&self, target: &Self) -> bool {
        self.center.distance(&target.center) < (self.radius + target.radius)
    }
}

#[derive(Debug)]
pub struct InnerLink<P, K: Center, const M: usize> {
    pub center_radius: Bounds<K>,
    // pub links: Box<Node<P, K, M>>,
    payload_type: std::marker::PhantomData<P>,
    pub links: u64,
}

#[derive(Debug)]
pub struct LeafLink<P, K: Center> {
    pub center_radius: Bounds<K>,
    pub payload: P,
}

#[derive(Debug)]
pub enum Node<P, K: Center, const M: usize> {
    Inner(ArrayVec<InnerLink<P, K, M>, M>),
    Leaf(ArrayVec<LeafLink<P, K>, M>),
}

#[derive(Debug)]
pub struct NodePool<P, K: Center, const M: usize> {
    nodes: HashMap<u64, Node<P, K, M>>,
    next_id: u64,
}

impl<P, K: Center, const M: usize> Default for NodePool<P, K, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P, K: Center, const M: usize> NodePool<P, K, M> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
        }
    }
}

impl<P, K: Center, const M: usize> NodePool<P, K, M> {
    pub fn alloc(&mut self) -> u64 {
        let ret = self.next_id;
        self.next_id += 1;
        // println!("alloc: {}", ret);
        ret
    }
    pub fn get(&self, id: u64) -> &Node<P, K, M> {
        // println!("get: {}", id);
        self.nodes.get(&id).expect("unknown node id")
    }
    pub fn remove(&mut self, id: u64) -> Node<P, K, M> {
        // println!("remove: {}", id);
        self.nodes.remove(&id).expect("unknown node id")
    }
    pub fn put(&mut self, id: u64, n: Node<P, K, M>) {
        // println!("put: {}", id);
        let _ = self.nodes.insert(id, n);
    }
}

impl<P, K: Center> AsRef<Bounds<K>> for LeafLink<P, K> {
    fn as_ref(&self) -> &Bounds<K> {
        &self.center_radius
    }
}

impl<P, K: Center> LeafLink<P, K> {
    pub fn new(center_radius: Bounds<K>, payload: P) -> Self {
        Self {
            center_radius,
            payload,
        }
    }
    pub fn intersects_point(&self, target: &K) -> bool {
        // self.center_radius.center.distance(target) <= self.center_radius.radius
        self.center_radius.intersects_point(target)
    }
}

impl<P, K: Center, const M: usize> AsRef<Bounds<K>> for InnerLink<P, K, M> {
    fn as_ref(&self) -> &Bounds<K> {
        &self.center_radius
    }
}

impl<P, K: Center, const M: usize> InnerLink<P, K, M> {
    pub fn from_entries(
        entries: ArrayVec<LeafLink<P, K>, M>,
        node_pool: &mut NodePool<P, K, M>,
    ) -> Self {
        let center_radius = util::centroid_and_radius(&entries);
        let node_id = node_pool.alloc();
        node_pool.put(node_id, Node::Leaf(entries));
        Self {
            center_radius,
            payload_type: PhantomData,
            links: node_id,
        }
    }

    pub fn from_nodes(nodes: ArrayVec<Self, M>, node_pool: &mut NodePool<P, K, M>) -> Self {
        let center_radius = util::centroid_and_radius(&nodes);
        let node_id = node_pool.alloc();
        node_pool.put(node_id, Node::Inner(nodes));
        Self {
            center_radius,
            payload_type: PhantomData,
            links: node_id,
        }
    }

    pub fn intersects_point(&self, target: &K) -> bool {
        // self.center_radius.center.distance(target) <= self.center_radius.radius
        self.center_radius.intersects_point(target)
    }

    pub fn search<'a>(&'a self, target: &K, pool: &'a NodePool<P, K, M>) -> Option<&Self> {
        let links = pool.get(self.links);
        match links {
            Node::Inner(children) => children.iter().find(|node| node.intersects_point(target)),
            Node::Leaf(points) => {
                if points.iter().any(|x| x.intersects_point(target)) {
                    Some(self)
                } else {
                    None
                }
            }
        }
    }

    pub fn search_parent_leaf<'a>(&'a self, target: &K, pool: &'a NodePool<P, K, M>) -> &Self {
        let links = pool.get(self.links);
        match links {
            Node::Inner(children) => {
                let child = Self::find_closest_child(children, target);
                child.search_parent_leaf(target, pool)
            }
            Node::Leaf(_) => self,
        }
    }

    // pub fn update_bounding_envelope(&mut self, pool: &NodePool<P, K, M>) {
    //     let links = pool.get(self.links);
    //     self.center_radius = match links {
    //         Node::Inner(nodes) => util::centroid_and_radius(nodes),
    //         Node::Leaf(points) => util::centroid_and_radius(points),
    //     };
    // }
    pub fn insert(
        &mut self,
        entry: LeafLink<P, K>,
        m: usize,
        pool: &mut NodePool<P, K, M>,
    ) -> Option<(Self, Self)> {
        let links = pool.remove(self.links);
        match links {
            Node::Leaf(mut points) => {
                if points.len() < M {
                    points.push(entry);
                    //self.update_bounding_envelope(pool);
                    self.center_radius = util::centroid_and_radius(&points);
                    self.links = pool.alloc();
                    pool.put(self.links, Node::Leaf(points));
                    None
                } else {
                    let mut nodes_to_split = points
                        .drain(..)
                        .chain(std::iter::once(entry))
                        .collect::<Vec<_>>();

                    let split_index = util::find_split_index(&mut nodes_to_split, m);
                    // let points2: ArrayVec<_, M> = nodes_to_split.drain(split_index..).collect();
                    // let center_radius2 = util::centroid_and_radius(&points2);

                    // let points1: ArrayVec<_, M> = nodes_to_split.drain(..split_index).collect();
                    // let center_radius1 = util::centroid_and_radius(&points1);

                    let new_node2 =
                        Self::from_entries(nodes_to_split.drain(split_index..).collect(), pool);

                    let new_node1 =
                        Self::from_entries(nodes_to_split.drain(..split_index).collect(), pool);

                    // let new_node1 = Self {
                    //     center_radius: center_radius1,
                    //     links: Box::new(Node::Leaf(points1)),
                    // };
                    // let new_node2 = Self {
                    //     center_radius: center_radius2,
                    //     links: Box::new(Node::Leaf(points2)),
                    // };

                    Some((new_node1, new_node2))
                }
            }

            Node::Inner(mut children) => {
                let closest_child_index =
                    Self::find_closest_child_index(&children, &entry.center_radius.center);
                if let Some((new_child_1, new_child_2)) =
                    children[closest_child_index].insert(entry, m, pool)
                {
                    children.remove(closest_child_index);

                    if children.len() < M - 1 {
                        children.push(new_child_1);
                        children.push(new_child_2);
                        self.center_radius = util::centroid_and_radius(&children);
                        self.links = pool.alloc();
                        pool.put(self.links, Node::Inner(children));
                        None
                    } else {
                        // TODO: use ArrayVec<_, M+1> when generic_const_exprs are suppported
                        let mut nodes_to_split: Vec<_> = children
                            .drain(..)
                            .chain(std::iter::once(new_child_1))
                            .chain(std::iter::once(new_child_2))
                            .collect();

                        let split_index = util::find_split_index(&mut nodes_to_split, m);

                        // let points2: ArrayVec<_, M> = nodes_to_split.drain(split_index..).collect();
                        // let center_radius2 = util::centroid_and_radius(&points2);

                        // let points1: ArrayVec<_, M> = nodes_to_split.drain(..split_index).collect();
                        // let center_radius1 = util::centroid_and_radius(&points1);

                        // let new_node1 = Self {
                        //     center_radius: center_radius1,
                        //     links: Box::new(Node::Inner(points1)),
                        // };
                        // let new_node2 = Self {
                        //     center_radius: center_radius2,
                        //     links: Box::new(Node::Inner(points2)),
                        // };
                        let new_node2 =
                            Self::from_nodes(nodes_to_split.drain(split_index..).collect(), pool);
                        let new_node1 =
                            Self::from_nodes(nodes_to_split.drain(..split_index).collect(), pool);

                        Some((new_node1, new_node2))
                    }
                } else {
                    // TODO: in case no child split happens we would not even need to remove self.links in the first place, but doing it like this keeps everything nice and uniform...
                    self.center_radius = util::centroid_and_radius(&children);
                    self.links = pool.alloc();
                    pool.put(self.links, Node::Inner(children));
                    None
                    // self.update_bounding_envelope(pool);
                }
            }
        }
    }

    pub fn remove(&mut self, target: &K, m: usize, pool: &mut NodePool<P, K, M>) -> (bool, bool) {
        let links = pool.remove(self.links);
        match links {
            Node::Leaf(mut entries) => {
                if let Some((i, _)) = entries
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.intersects_point(target))
                {
                    entries.remove(i);
                    let num_entries = entries.len();
                    if num_entries != 0 {
                        self.center_radius = util::centroid_and_radius(&entries);
                        // update_bounding_envelope();
                    }
                    self.links = pool.alloc();
                    pool.put(self.links, Node::Leaf(entries));

                    (true, num_entries < m)
                } else {
                    self.links = pool.alloc();
                    pool.put(self.links, Node::Leaf(entries));
                    (false, false)
                }
            }
            Node::Inner(mut nodes) => {
                let mut node_to_fix_index = None;
                let mut deleted = false;
                for (i, child_node) in nodes.iter_mut().enumerate() {
                    if child_node.intersects_point(target) {
                        let res = child_node.remove(target, m, pool);
                        deleted = res.0;
                        let violates_invariants = res.1;
                        // println!("{:?} {:?}", deleted, violates_invariants);
                        if violates_invariants {
                            node_to_fix_index = Some(i);
                        }
                        if deleted {
                            break;
                        }
                    }
                }
                match node_to_fix_index {
                    None => {
                        if deleted {
                            //self.update_bounding_envelope();
                            self.center_radius = util::centroid_and_radius(&nodes);
                        }
                        self.links = pool.alloc();
                        pool.put(self.links, Node::Inner(nodes));
                        (deleted, false)
                    }

                    Some(node_to_fix) => {
                        if let Some(sibling_to_borrow_from) =
                            Self::find_sibling_to_borrow_from(&nodes, node_to_fix, m, pool)
                        {
                            Self::borrow_from_sibling(
                                &mut nodes,
                                node_to_fix,
                                sibling_to_borrow_from,
                                pool,
                            );
                        } else if let Some(sibling_to_merge_to) =
                            Self::find_sibling_to_merge_to(&nodes, node_to_fix, m, pool)
                        {
                            // no sibling to borrow from -> merge
                            Self::merge_siblings(
                                &mut nodes,
                                node_to_fix,
                                sibling_to_merge_to,
                                pool,
                            );
                        }
                        let num_nodes = nodes.len();
                        if num_nodes != 0 {
                            self.center_radius = util::centroid_and_radius(&nodes);
                            //update_bounding_envelope();
                        }
                        self.links = pool.alloc();
                        pool.put(self.links, Node::Inner(nodes));
                        (true, num_nodes < m)
                    }
                }
            }
        }
    }

    pub fn count_nodes(&self, pool: &NodePool<P, K, M>) -> (usize, usize) {
        let links = pool.get(self.links);
        match links {
            Node::Inner(nodes) => nodes.iter().fold((0, 1), |(a_points, a_nodes), n| {
                let (points, nodes) = n.count_nodes(pool);
                (a_points + points, a_nodes + nodes)
            }),
            Node::Leaf(points) => (points.len(), 1),
        }
    }
    pub fn find_entries_within_radius<'a>(
        &'a self,
        // center: &K,
        // radius: f32,
        center_radius: &Bounds<K>,
        out: &mut Vec<&'a LeafLink<P, K>>,
        pool: &'a NodePool<P, K, M>,
    ) {
        let links = pool.get(self.links);
        match links {
            Node::Leaf(points) => {
                for point in points.iter() {
                    if point.center_radius.intersects(center_radius) {
                        out.push(point);
                    }
                }
            }
            Node::Inner(nodes) => {
                for child in nodes.iter() {
                    if child.center_radius.intersects(center_radius) {
                        child.find_entries_within_radius(center_radius, out, pool);
                    }
                }
            }
        }
    }

    pub fn find_if<'a, F: Fn(&P) -> bool>(
        &'a self,
        center_radius: &Bounds<K>,
        f: &F,
        pool: &'a NodePool<P, K, M>,
    ) -> Option<&LeafLink<P, K>> {
        let links = pool.get(self.links);
        match links {
            Node::Leaf(points) => {
                for (_i, point) in points.iter().enumerate() {
                    if point.center_radius.intersects(center_radius) && f(&point.payload) {
                        return Some(point);
                    }
                }
            }
            Node::Inner(nodes) => {
                for (_i, child) in nodes.iter().enumerate() {
                    if child.center_radius.intersects(center_radius) {
                        let ret = child.find_if(center_radius, f, pool);
                        if ret.is_some() {
                            return ret;
                        }
                    }
                }
            }
        }
        None
    }

    pub fn remove_if<F: Fn(&P) -> bool>(
        &mut self,
        center_radius: &Bounds<K>,
        m: usize,
        f: &F,
        pool: &mut NodePool<P, K, M>,
    ) -> (bool, bool, Option<LeafLink<P, K>>) {
        let links = pool.remove(self.links);
        match links {
            Node::Leaf(mut entries) => {
                if let Some((i, _)) = entries
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.center_radius.intersects(center_radius) && f(&p.payload))
                {
                    let e = entries.remove(i);
                    let num_entries = entries.len();
                    if num_entries != 0 {
                        self.center_radius = util::centroid_and_radius(&entries);
                        // update_bounding_envelope();
                    }
                    self.links = pool.alloc();
                    pool.put(self.links, Node::Leaf(entries));
                    (true, num_entries < m, Some(e))
                } else {
                    self.links = pool.alloc();
                    pool.put(self.links, Node::Leaf(entries));
                    (false, false, None)
                }
            }
            Node::Inner(mut nodes) => {
                let mut node_to_fix_index = None;
                let mut deleted = false;
                let mut deleted_entry = None;
                for (i, child_node) in nodes.iter_mut().enumerate() {
                    if child_node.center_radius.intersects(center_radius) {
                        let res = child_node.remove_if(center_radius, m, f, pool); // FIXME: ignoring radius
                        deleted = res.0;
                        let violates_invariants = res.1;
                        // println!("{:?} {:?}", deleted, violates_invariants);
                        if violates_invariants {
                            node_to_fix_index = Some(i);
                        }
                        if deleted {
                            deleted_entry = res.2;
                            break;
                        }
                    }
                }
                match node_to_fix_index {
                    None => {
                        if deleted {
                            self.center_radius = util::centroid_and_radius(&nodes);
                            // update_bounding_envelope();
                        }
                        self.links = pool.alloc();
                        pool.put(self.links, Node::Inner(nodes));
                        (deleted, false, deleted_entry)
                    }

                    Some(node_to_fix) => {
                        if let Some(sibling_to_borrow_from) =
                            Self::find_sibling_to_borrow_from(&nodes, node_to_fix, m, pool)
                        {
                            Self::borrow_from_sibling(
                                &mut nodes,
                                node_to_fix,
                                sibling_to_borrow_from,
                                pool,
                            );
                        } else if let Some(sibling_to_merge_to) =
                            Self::find_sibling_to_merge_to(&nodes, node_to_fix, m, pool)
                        {
                            // no sibling to borrow from -> merge
                            Self::merge_siblings(
                                &mut nodes,
                                node_to_fix,
                                sibling_to_merge_to,
                                pool,
                            );
                        }
                        let num_nodes = nodes.len();
                        if num_nodes != 0 {
                            self.center_radius = util::centroid_and_radius(&nodes);
                            //update_bounding_envelope();
                        }
                        self.links = pool.alloc();
                        pool.put(self.links, Node::Inner(nodes));

                        (true, num_nodes < m, deleted_entry)
                    }
                }
            }
        }
    }

    fn find_closest_child<'a>(children: &'a [Self], target: &K) -> &'a Self {
        let mut min_dist = f32::MAX;
        let mut cur_min = None;
        for child in children {
            let d = child.center_radius.distance_point(target);
            if d < min_dist {
                min_dist = d;
                cur_min = Some(child);
            }
        }
        cur_min.unwrap()
    }
    fn find_closest_child_index(children: &[Self], target: &K) -> usize {
        let mut min_dist = f32::MAX;
        let mut cur_min = None;
        for (i, child) in children.iter().enumerate() {
            let d = child.center_radius.distance_point(target);
            if d < min_dist {
                min_dist = d;
                cur_min = Some(i);
            }
        }
        cur_min.unwrap()
    }

    fn find_sibling_to_borrow_from(
        nodes: &[Self],
        node_to_fix: usize,
        m: usize,
        pool: &NodePool<P, K, M>,
    ) -> Option<usize> {
        let siblings_to_borrow_from = nodes.iter().enumerate().filter(|(i, sibling)| {
            let links = pool.get(sibling.links);
            match links {
                Node::Inner(nodes) => *i != node_to_fix && nodes.len() > m,
                Node::Leaf(points) => *i != node_to_fix && points.len() > m,
            }
        });

        let mut closest_sibling = None;
        let mut closest_sibling_dist = f32::INFINITY;

        for (i, sibling) in siblings_to_borrow_from {
            let distance = nodes[node_to_fix]
                .center_radius
                .distance(&sibling.center_radius);
            if distance < closest_sibling_dist {
                closest_sibling = Some(i);
                closest_sibling_dist = distance;
            }
        }
        closest_sibling
    }

    fn borrow_from_sibling(
        nodes: &mut [Self],
        node_to_fix: usize,
        sibling_to_borrow_from: usize,
        pool: &mut NodePool<P, K, M>,
    ) {
        // found sibling to borrow from
        let to_fix_centroid = &nodes[node_to_fix].center_radius.center;
        let links = pool.remove(nodes[sibling_to_borrow_from].links);
        match links {
            Node::Inner(mut nodes2) => {
                let mut closest_node = None;
                let mut closest_node_dist = f32::INFINITY;
                for (i, node) in nodes2.iter().enumerate() {
                    let distance = node.center_radius.distance_point(to_fix_centroid);
                    if distance < closest_node_dist {
                        closest_node = Some(i);
                        closest_node_dist = distance;
                    }
                }
                let node = nodes2.remove(closest_node.unwrap());
                // nodes[sibling_to_borrow_from].update_bounding_envelope(pool);
                nodes[sibling_to_borrow_from].center_radius = util::centroid_and_radius(&nodes2);

                let links = pool.remove(nodes[node_to_fix].links);
                match links {
                    Node::Inner(mut fix_nodes) => {
                        fix_nodes.push(node);
                        nodes[node_to_fix].center_radius = util::centroid_and_radius(&fix_nodes);
                        nodes[node_to_fix].links = pool.alloc();
                        pool.put(nodes[node_to_fix].links, Node::Inner(fix_nodes));
                    }
                    Node::Leaf(_) => panic!("unbalanced tree"),
                }
                // nodes[node_to_fix].update_bounding_envelope();
                nodes[sibling_to_borrow_from].links = pool.alloc();
                pool.put(nodes[sibling_to_borrow_from].links, Node::Inner(nodes2));
            }
            Node::Leaf(mut points) => {
                let mut closest_point = None;
                let mut closest_point_dist = f32::INFINITY;
                for (i, point) in points.iter().enumerate() {
                    let distance = point.center_radius.distance_point(to_fix_centroid);
                    if distance < closest_point_dist {
                        closest_point = Some(i);
                        closest_point_dist = distance;
                    }
                }
                // println!(
                //     "closest point: {:?} {} {}",
                //     closest_point, sibling_to_borrow_from, node_to_fix
                // );
                let point = points.remove(closest_point.unwrap());
                // nodes[sibling_to_borrow_from].update_bounding_envelope();
                nodes[sibling_to_borrow_from].center_radius = util::centroid_and_radius(&points);
                let links = pool.remove(nodes[node_to_fix].links);

                match links {
                    Node::Inner(_) => panic!("unbalanced tree"),
                    Node::Leaf(mut fix_points) => {
                        fix_points.push(point);
                        nodes[node_to_fix].center_radius = util::centroid_and_radius(&fix_points);
                        nodes[node_to_fix].links = pool.alloc();
                        pool.put(nodes[node_to_fix].links, Node::Leaf(fix_points));
                    }
                }
                // nodes[node_to_fix].update_bounding_envelope();
                nodes[sibling_to_borrow_from].links = pool.alloc();
                pool.put(nodes[sibling_to_borrow_from].links, Node::Leaf(points));
            }
        }
    }

    fn find_sibling_to_merge_to(
        nodes: &[Self],
        node_to_fix: usize,
        m: usize,
        pool: &NodePool<P, K, M>,
    ) -> Option<usize> {
        let siblings_to_merge_to = nodes.iter().enumerate().filter(|(i, sibling)| {
            let links = pool.get(sibling.links);
            match links {
                Node::Inner(nodes) => *i != node_to_fix && nodes.len() == m,
                Node::Leaf(points) => *i != node_to_fix && points.len() == m,
            }
        });

        let mut closest_sibling = None;
        let mut closest_sibling_dist = f32::INFINITY;

        for (i, sibling) in siblings_to_merge_to {
            let distance = nodes[node_to_fix]
                .center_radius
                .distance(&sibling.center_radius);
            if distance < closest_sibling_dist {
                closest_sibling = Some(i);
                closest_sibling_dist = distance;
            }
        }
        closest_sibling
    }

    fn merge_siblings(
        nodes: &mut ArrayVec<Self, M>,
        mut node_index_1: usize,
        mut node_index_2: usize,
        pool: &mut NodePool<P, K, M>,
    ) {
        if node_index_1 > node_index_2 {
            // remove node with larger index first
            std::mem::swap(&mut node_index_1, &mut node_index_2);
        }
        let node_2 = nodes.remove(node_index_2);
        let node_1 = nodes.remove(node_index_1);
        let node = Self::merge(node_1, node_2, pool);
        nodes.push(node);
    }

    fn merge(node_1: Self, node_2: Self, pool: &mut NodePool<P, K, M>) -> Self {
        let links_1 = pool.remove(node_1.links);
        let links_2 = pool.remove(node_2.links);
        match (links_1, links_2) {
            (Node::Leaf(mut points1), Node::Leaf(mut points2)) => {
                points1.extend(points2.drain(..));
                InnerLink::<P, K, M>::from_entries(points1, pool)
            }
            (Node::Inner(mut nodes1), Node::Inner(mut nodes2)) => {
                nodes1.extend(nodes2.drain(..));
                InnerLink::<P, K, M>::from_nodes(nodes1, pool)
            }
            _ => panic!("inconsistent siblings"),
        }
    }

    // function pointsWithinRegion(node, region)
    //   points ← []
    //   if node.leaf then
    //     for point in node.points do
    //       if region.intersectsPoint(point) then
    //         points.insert(point)
    //   else
    //    for child in node.children do
    //       if region.intersectsNode(child) then
    //         points.insertAll(pointsWithinRegion(child, region))
    //   return points
}

#[derive(Debug)]
pub struct SsTree<P, K: Center, const M: usize> {
    pub root: InnerLink<P, K, M>,
    pub pool: NodePool<P, K, M>,
    height: usize,
    m: usize,
}

impl<P, K: Center, const M: usize> SsTree<P, K, M> {
    pub fn new(m: usize) -> Self {
        let mut pool = NodePool::new();
        let links = pool.alloc();
        pool.put(links, Node::Leaf(ArrayVec::new()));
        Self {
            root: InnerLink {
                center_radius: Bounds::from_center_radius(K::default(), 0f32),
                links, //: Box::new(Node::Leaf(ArrayVec::new())),
                payload_type: PhantomData,
            },
            pool,

            height: 1,
            m,
        }
    }

    pub fn insert(&mut self, payload: P, center: K, radius: f32) {
        self.insert_entry(LeafLink {
            center_radius: Bounds::from_center_radius(center, radius),
            payload,
        })
    }
    pub fn insert_entry(&mut self, entry: LeafLink<P, K>) {
        if let Some((new_child_1, new_child_2)) = self.root.insert(entry, self.m, &mut self.pool) {
            let mut nodes = ArrayVec::<_, M>::new();
            nodes.push(new_child_1);
            nodes.push(new_child_2);
            let center_radius = util::centroid_and_radius(&nodes);
            let links = self.pool.alloc();
            self.pool.put(links, Node::Inner(nodes));
            self.root = InnerLink {
                center_radius,
                links,
                payload_type: PhantomData,
            };
            self.height += 1;
        }
    }

    #[allow(clippy::overly_complex_bool_expr)]
    pub fn remove(&mut self, point: &K) {
        let (_deleted, _violiates_invariant) = self.root.remove(point, self.m, &mut self.pool);

        let links = self.pool.get(self.root.links);

        match links {
            Node::Inner(nodes) if nodes.len() == 1 => {
                let links = self.pool.remove(self.root.links);
                let Node::Inner(mut nodes) =  links else { panic!("expecting Node::Inner here") };
                self.root = nodes.pop().unwrap();
                self.height -= 1;
            }
            _ => (),
        }
    }

    pub fn get_height(&self) -> usize {
        self.height
    }
    pub fn get_fill_factor(&self) -> f32 {
        let (num_points, num_nodes) = self.root.count_nodes(&self.pool);
        num_points as f32 / num_nodes as f32
    }

    pub fn find_entries_within_radius<'a>(
        &'a self,
        center_radius: &Bounds<K>,
        out: &mut Vec<&'a LeafLink<P, K>>,
    ) {
        self.root
            .find_entries_within_radius(center_radius, out, &self.pool);
    }

    pub fn find_if<'a, F: Fn(&P) -> bool>(
        &'a self,
        center_radius: &Bounds<K>,
        f: F,
    ) -> Option<&'a LeafLink<P, K>> {
        self.root.find_if(center_radius, &f, &self.pool)
    }
    pub fn remove_if<F: Fn(&P) -> bool>(
        &mut self,
        center_radius: &Bounds<K>,
        f: F,
    ) -> Option<LeafLink<P, K>> {
        let deleted_entry = self
            .root
            .remove_if(center_radius, self.m, &f, &mut self.pool)
            .2;
        let links = self.pool.get(self.root.links);
        match links {
            Node::Inner(nodes) if nodes.len() == 1 => {
                let links = self.pool.remove(self.root.links);
                let Node::Inner(mut nodes) =  links else { panic!("expecting Node::Inner here") };
                self.root = nodes.pop().unwrap();
                self.height -= 1;
            }
            _ => (),
        }
        deleted_entry
    }
}

impl<P, K: Center, const M: usize> Default for SsTree<P, K, M> {
    fn default() -> Self {
        Self::new(M / 2)
    }
}

mod util {
    use super::{Bounds, Center};

    pub fn mean_along_direction<K: Center>(
        entry: &[impl AsRef<Bounds<K>>],
        direction_index: usize,
    ) -> f32 {
        assert!(!entry.is_empty());
        let count = entry.len() as f32;
        let sum = entry
            .iter()
            .map(|point| point.as_ref().center[direction_index])
            .sum::<f32>();
        sum / count
    }

    pub fn variance_along_direction<K: Center>(
        entries: &[impl AsRef<Bounds<K>>],
        direction_index: usize,
    ) -> f32 {
        assert!(!entries.is_empty());
        let mean = mean_along_direction(entries, direction_index);
        let count = entries.len() as f32;
        entries
            .iter()
            .map(|point| {
                let diff = mean - point.as_ref().center[direction_index];
                diff * diff
            })
            .sum::<f32>()
            / count
    }

    pub fn direction_of_max_variance<K: Center>(entries: &[impl AsRef<Bounds<K>>]) -> usize {
        let mut max_variance = 0.0;
        let mut direction_index = 0;
        for i in 0..K::NUM_DIMENSIONS {
            let variance = variance_along_direction(entries, i);
            if variance > max_variance {
                max_variance = variance;
                direction_index = i;
            }
        }
        direction_index
    }

    pub fn centroid<K: Center>(entries: &[impl AsRef<Bounds<K>>]) -> K {
        let mut centroid = K::default();
        for i in 0..K::NUM_DIMENSIONS {
            centroid[i] = mean_along_direction(entries, i);
        }
        centroid
    }

    pub fn centroid_and_radius<K: Center>(nodes: &[impl AsRef<Bounds<K>>]) -> Bounds<K> {
        let centroid = centroid::<K>(nodes);
        let radius = nodes
            .iter()
            .map(|node| centroid.distance(&node.as_ref().center) + node.as_ref().radius)
            .max_by(|d1, d2| d1.partial_cmp(d2).unwrap())
            .unwrap();
        Bounds::from_center_radius(centroid, radius)
    }

    pub fn find_split_index<K: Center>(nodes: &mut [impl AsRef<Bounds<K>>], m: usize) -> usize {
        let coordinate_index = direction_of_max_variance::<K>(nodes);
        nodes.sort_by(|p1, p2| {
            p1.as_ref().center[coordinate_index]
                .partial_cmp(&p2.as_ref().center[coordinate_index])
                .unwrap()
        });

        let mut min_variance = f32::INFINITY;
        let mut split_index = m;
        for i in m..=(nodes.len() - m) {
            let variance1 = variance_along_direction(&nodes[..i], coordinate_index);
            let variance2 = variance_along_direction(&nodes[i..], coordinate_index);
            let variance = variance1 + variance2;
            if variance < min_variance {
                min_variance = variance;
                split_index = i;
            }
        }
        split_index
    }
}

impl<const K: usize> Distance for [f32; K] {
    fn distance(&self, p2: &[f32; K]) -> f32 {
        self.iter()
            .zip(p2.iter())
            .map(|(c1, c2)| (c1 - c2) * (c1 - c2))
            .sum::<f32>()
            .sqrt()
    }
}

impl<const K: usize> DimIndex for [f32; K] {
    const NUM_DIMENSIONS: usize = K;
}

impl Distance for Vec3 {
    fn distance(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }
}

impl DimIndex for Vec3 {
    const NUM_DIMENSIONS: usize = 3;
}

#[test]
fn test_distance() {
    assert_eq!([0.0, 0.0].distance(&[1.0, 1.0]), 2.0f32.sqrt());
    assert_eq!([-10.0, 1.0].distance(&[10.0, 1.0]), 20.0f32);
    assert_eq!([1000.0, -1000.0].distance(&[1000.0, 2000.0]), 3000.0);
}

#[test]
fn test_bevy_vec3() {
    let mut tree = SsTree::<u32, Vec3, 8>::default();
    let a = Vec3::ZERO;
    println!("{}", a[0]);
    tree.insert_entry(LeafLink {
        payload: 1,
        center_radius: Bounds {
            center: Vec3::ZERO,
            radius: 1.0,
        },
    });
}

#[cfg(test)]
mod test {
    use super::Bounds;

    use super::LeafLink;
    use super::SsTree;

    // #[derive(Debug)]
    // struct CenterRadius2 {
    //     center: [f32; 2],
    //     radius: f32,
    // }

    // impl CenterRadius for CenterRadius2 {
    //     type K = [f32; 2];

    //     fn center(&self) -> &Self::K {
    //         &self.center
    //     }

    //     fn from_center_radius(center: Self::K, radius: f32) -> Self {
    //         Self { center, radius }
    //     }

    //     fn radius(&self) -> f32 {
    //         self.radius
    //     }
    // }

    impl<P> PartialEq for LeafLink<P, [f32; 2]> {
        fn eq(&self, other: &Self) -> bool {
            self.center_radius.center == other.center_radius.center
                && self.center_radius.radius == other.center_radius.radius
        }
    }

    #[test]
    fn test_search() {
        const UPPER_M: usize = 8;
        const LOWER_M: usize = 4;

        let mut tree = SsTree::<(), [f32; 2], UPPER_M>::new(LOWER_M);

        tree.insert_entry(LeafLink::new(
            Bounds {
                center: [0.0, 0.0],
                radius: 1.0,
            },
            (),
        ));
        tree.insert_entry(LeafLink::new(
            Bounds {
                center: [5.0, 5.0],
                radius: 1.0,
            },
            (),
        ));

        let mut out = Vec::new();
        tree.find_entries_within_radius(
            &Bounds {
                center: [0.5, 0.5],
                radius: 1.0,
            },
            &mut out,
        );
        assert_eq!(
            out,
            vec!(&LeafLink::new(
                Bounds {
                    center: [0.0, 0.0],
                    radius: 1.0,
                },
                ()
            ))
        );

        let mut out = Vec::new();
        tree.find_entries_within_radius(
            &Bounds {
                center: [4.5, 5.5],
                radius: 1.0,
            },
            &mut out,
        );
        assert_eq!(
            out,
            vec!(&LeafLink::new(
                Bounds {
                    center: [5.0, 5.0],
                    radius: 1.0,
                },
                ()
            ))
        );
        let mut out = Vec::new();

        // do search between the entries with radius big enough to just reach them
        tree.find_entries_within_radius(
            &Bounds {
                center: [2.5, 2.5],
                radius: (2.5 * std::f32::consts::SQRT_2 + 0.0001) - 1.0,
            },
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert!(out.contains(&&LeafLink::<(), _>::new(
            Bounds {
                center: [5.0, 5.0],
                radius: 1.0,
            },
            ()
        )));
        assert!(out.contains(&&LeafLink::<(), _>::new(
            Bounds {
                center: [0.0, 0.0],
                radius: 1.0,
            },
            ()
        )));

        let mut out = Vec::new();

        // the same as befor but with radius just barely too small
        tree.find_entries_within_radius(
            &Bounds {
                center: [2.5, 2.5],
                radius: (2.5 * std::f32::consts::SQRT_2 - 0.0001) - 1.0,
            },
            &mut out,
        );
        assert!(out.is_empty());
    }
}

use bevy::prelude::*;

type SpatialBounds = Bounds<Vec3>;

#[derive(Resource, Default)]
pub struct SpatialIndex {
    sstree: SsTree<Entity, Vec3, 8>,
}

impl SpatialIndex {
    pub fn clear(&mut self) {
        self.sstree = SsTree::default();
    }
    pub fn update(&mut self, entity: Entity, from: Option<SpatialBounds>, to: SpatialBounds) {
        if let Some(center_radius) = from {
            if self
                .sstree
                .remove_if(&center_radius, |e| *e == entity)
                .is_none()
            {
                error!("failed to remove brush from spatial index for update");
                panic!("aborting");
            }
        }
        self.sstree.insert(entity, to.center, to.radius);
    }

    pub fn remove(&mut self, entity: Entity, bounds: SpatialBounds) {
        self.sstree.remove_if(&bounds, |e| *e == entity);
    }

    pub fn query(&self, bounds: SpatialBounds) -> impl Iterator<Item = Entity> + '_ {
        let mut out = Vec::new();
        self.sstree.find_entries_within_radius(&bounds, &mut out);
        out.into_iter().map(|e| e.payload)
    }
}
