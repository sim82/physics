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

#[derive(Debug)]
pub struct CenterRadius<K> {
    pub center: K,
    pub radius: f32,
}

#[derive(Debug)]
pub struct Entry<P, K> {
    pub center_radius: CenterRadius<K>,
    pub payload: P,
}

impl<P, K: Distance> Entry<P, K> {
    pub fn new(center: K, radius: f32, payload: P) -> Self {
        Self {
            center_radius: CenterRadius { center, radius },
            payload,
        }
    }
    pub fn intersects_point(&self, target: &K) -> bool {
        self.center_radius.center.distance(target) <= self.center_radius.radius
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

#[derive(Debug)]
pub enum SsNodeLinks<P, K: Distance + DimIndex + PartialEq, const M: usize> {
    Inner(Box<ArrayVec<SsNode<P, K, M>, M>>),
    Leaf(Box<ArrayVec<Entry<P, K>, M>>),
}

#[test]
fn test_bevy_vec3() {
    let mut tree = SsTree::<u32, Vec3, 8>::default();
    let a = Vec3::ZERO;
    println!("{}", a[0]);
    tree.insert_entry(Entry {
        payload: 1,
        center_radius: CenterRadius {
            center: Vec3::ZERO,
            radius: 1.0,
        },
    });
}

#[derive(Debug)]
pub struct SsNode<P, K: Distance + DimIndex + PartialEq, const M: usize> {
    // pub centroid: K,
    // pub radius: f32,
    pub center_radius: CenterRadius<K>,
    pub links: SsNodeLinks<P, K, M>,
}

impl<P, K: Default + DimIndex + Distance + PartialEq, const M: usize> SsNode<P, K, M> {
    pub fn from_entries(entries: ArrayVec<Entry<P, K>, M>) -> Self {
        let (centroid, radius) = leaf::centroid_and_radius::<P, K, M>(&entries);
        Self {
            center_radius: CenterRadius {
                center: centroid,
                radius,
            },
            links: SsNodeLinks::Leaf(Box::new(entries)),
        }
    }

    pub fn from_nodes(nodes: ArrayVec<Self, M>) -> Self {
        let (centroid, radius) = inner::centroid_and_radius(&nodes);
        Self {
            center_radius: CenterRadius {
                center: centroid,
                radius,
            },
            links: SsNodeLinks::Inner(Box::new(nodes)),
        }
    }

    pub fn intersects_point(&self, target: &K) -> bool {
        self.center_radius.center.distance(target) <= self.center_radius.radius
    }

    pub fn search(&self, target: &K) -> Option<&Self> {
        match &self.links {
            SsNodeLinks::Inner(children) => {
                children.iter().find(|node| node.intersects_point(target))
            }
            SsNodeLinks::Leaf(points) => {
                if points.iter().any(|x| x.intersects_point(target)) {
                    Some(self)
                } else {
                    None
                }
            }
        }
    }

    pub fn search_parent_leaf(&self, target: &K) -> &Self {
        match &self.links {
            SsNodeLinks::Inner(children) => {
                let child = find_closest_child(children, target);
                child.search_parent_leaf(target)
            }
            SsNodeLinks::Leaf(_) => self,
        }
    }

    pub fn update_bounding_envelope(&mut self) {
        let (centroid, radius) = match &self.links {
            SsNodeLinks::Inner(nodes) => inner::centroid_and_radius(nodes),
            SsNodeLinks::Leaf(points) => leaf::centroid_and_radius::<P, K, M>(points),
        };
        self.center_radius.center = centroid;
        self.center_radius.radius = radius;
    }
    pub fn insert(&mut self, entry: Entry<P, K>, m: usize) -> Option<(Self, Self)> {
        match &mut self.links {
            SsNodeLinks::Leaf(points) => {
                if points.len() < M {
                    points.push(entry);
                    self.update_bounding_envelope();
                    return None;
                } else {
                    let mut nodes_to_split = points
                        .drain(..)
                        .chain(std::iter::once(entry))
                        .collect::<Vec<_>>();

                    let split_index = leaf::find_split_index::<P, K, M>(&mut nodes_to_split, m);
                    let points2: ArrayVec<_, M> = nodes_to_split.drain(split_index..).collect();
                    let (centroid2, radius2) = leaf::centroid_and_radius::<P, K, M>(&points2);

                    let points1: ArrayVec<_, M> = nodes_to_split.drain(..split_index).collect();
                    let (centroid1, radius1) = leaf::centroid_and_radius::<P, K, M>(&points1);

                    let new_node1 = Self {
                        center_radius: CenterRadius {
                            center: centroid1,
                            radius: radius1,
                        },
                        links: SsNodeLinks::Leaf(Box::new(points1)),
                    };
                    let new_node2 = Self {
                        center_radius: CenterRadius {
                            center: centroid2,
                            radius: radius2,
                        },
                        links: SsNodeLinks::Leaf(Box::new(points2)),
                    };

                    return Some((new_node1, new_node2));
                }
            }

            SsNodeLinks::Inner(children) => {
                let closest_child_index =
                    find_closest_child_index(children, &entry.center_radius.center);
                if let Some((new_child_1, new_child_2)) =
                    children[closest_child_index].insert(entry, m)
                {
                    children.remove(closest_child_index);

                    if children.len() < M - 1 {
                        children.push(new_child_1);
                        children.push(new_child_2);
                    } else {
                        let mut nodes_to_split: Vec<_> = children
                            .drain(..)
                            .chain(std::iter::once(new_child_1))
                            .chain(std::iter::once(new_child_2))
                            .collect();

                        let split_index = inner::find_split_index(&mut nodes_to_split, m);

                        let points2: ArrayVec<_, M> = nodes_to_split.drain(split_index..).collect();
                        let (centroid2, radius2) = inner::centroid_and_radius(&points2);

                        let points1: ArrayVec<_, M> = nodes_to_split.drain(..split_index).collect();
                        let (centroid1, radius1) = inner::centroid_and_radius(&points1);

                        let new_node1 = Self {
                            center_radius: CenterRadius {
                                center: centroid1,
                                radius: radius1,
                            },
                            links: SsNodeLinks::Inner(Box::new(points1)),
                        };
                        let new_node2 = Self {
                            center_radius: CenterRadius {
                                center: centroid2,
                                radius: radius2,
                            },
                            links: SsNodeLinks::Inner(Box::new(points2)),
                        };
                        return Some((new_node1, new_node2));
                    }
                } else {
                    self.update_bounding_envelope();
                }
            }
        }
        None
    }

    pub fn remove(&mut self, target: &K, m: usize) -> (bool, bool) {
        match &mut self.links {
            SsNodeLinks::Leaf(entries) => {
                if let Some((i, _)) = entries
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.intersects_point(target))
                {
                    entries.remove(i);
                    let num_entries = entries.len();
                    if num_entries != 0 {
                        self.update_bounding_envelope();
                    }
                    (true, num_entries < m)
                } else {
                    (false, false)
                }
            }
            SsNodeLinks::Inner(nodes) => {
                let mut node_to_fix_index = None;
                let mut deleted = false;
                for (i, child_node) in nodes.iter_mut().enumerate() {
                    if child_node.intersects_point(target) {
                        let res = child_node.remove(target, m);
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
                            self.update_bounding_envelope();
                        }
                        (deleted, false)
                    }

                    Some(node_to_fix) => {
                        if let Some(sibling_to_borrow_from) =
                            inner::find_sibling_to_borrow_from(nodes, node_to_fix, m)
                        {
                            inner::borrow_from_sibling(nodes, node_to_fix, sibling_to_borrow_from);
                        } else if let Some(sibling_to_merge_to) =
                            inner::find_sibling_to_merge_to(nodes, node_to_fix, m)
                        {
                            // no sibling to borrow from -> merge
                            inner::merge_siblings(nodes, node_to_fix, sibling_to_merge_to);
                        }
                        let num_nodes = nodes.len();
                        if num_nodes != 0 {
                            self.update_bounding_envelope();
                        }

                        (true, num_nodes < m)
                    }
                }
            }
        }
    }

    pub fn count_nodes(&self) -> (usize, usize) {
        match &self.links {
            SsNodeLinks::Inner(nodes) => nodes.iter().fold((0, 1), |(a_points, a_nodes), n| {
                let (points, nodes) = n.count_nodes();
                (a_points + points, a_nodes + nodes)
            }),
            SsNodeLinks::Leaf(points) => (points.len(), 1),
        }
    }
    pub fn find_entries_within_radius<'a>(
        &'a self,
        center: &K,
        radius: f32,
        out: &mut Vec<&'a Entry<P, K>>,
    ) {
        match &self.links {
            SsNodeLinks::Leaf(points) => {
                for point in points.iter() {
                    if point.center_radius.center.distance(center)
                        < (radius + point.center_radius.radius)
                    {
                        out.push(point);
                    }
                }
            }
            SsNodeLinks::Inner(nodes) => {
                for child in nodes.iter() {
                    if child.center_radius.center.distance(center)
                        <= radius + child.center_radius.radius
                    {
                        child.find_entries_within_radius(center, radius, out);
                    }
                }
            }
        }
    }

    pub fn find_if<F: Fn(&P) -> bool>(
        &self,
        center: &K,
        radius: f32,
        f: &F,
    ) -> Option<&Entry<P, K>> {
        match &self.links {
            SsNodeLinks::Leaf(points) => {
                for (_i, point) in points.iter().enumerate() {
                    if point.center_radius.center.distance(center)
                        < (radius + point.center_radius.radius)
                        && f(&point.payload)
                    {
                        return Some(point);
                    }
                }
            }
            SsNodeLinks::Inner(nodes) => {
                for (_i, child) in nodes.iter().enumerate() {
                    if child.center_radius.center.distance(center)
                        <= radius + child.center_radius.radius
                    {
                        let ret = child.find_if(center, radius, f);
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
        center: &K,
        radius: f32,
        m: usize,
        f: &F,
    ) -> (bool, bool, Option<Entry<P, K>>) {
        match &mut self.links {
            SsNodeLinks::Leaf(entries) => {
                if let Some((i, _)) = entries.iter().enumerate().find(|(_, p)| {
                    p.center_radius.center.distance(center) < (radius + p.center_radius.radius)
                        && f(&p.payload)
                }) {
                    let e = entries.remove(i);
                    let num_entries = entries.len();
                    if num_entries != 0 {
                        self.update_bounding_envelope();
                    }
                    (true, num_entries < m, Some(e))
                } else {
                    (false, false, None)
                }
            }
            SsNodeLinks::Inner(nodes) => {
                let mut node_to_fix_index = None;
                let mut deleted = false;
                let mut deleted_entry = None;
                for (i, child_node) in nodes.iter_mut().enumerate() {
                    if child_node.center_radius.center.distance(center)
                        <= radius + child_node.center_radius.radius
                    {
                        let res = child_node.remove_if(center, radius, m, f); // FIXME: ignoring radius
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
                            self.update_bounding_envelope();
                        }
                        (deleted, false, deleted_entry)
                    }

                    Some(node_to_fix) => {
                        if let Some(sibling_to_borrow_from) =
                            inner::find_sibling_to_borrow_from(nodes, node_to_fix, m)
                        {
                            inner::borrow_from_sibling(nodes, node_to_fix, sibling_to_borrow_from);
                        } else if let Some(sibling_to_merge_to) =
                            inner::find_sibling_to_merge_to(nodes, node_to_fix, m)
                        {
                            // no sibling to borrow from -> merge
                            inner::merge_siblings(nodes, node_to_fix, sibling_to_merge_to);
                        }
                        let num_nodes = nodes.len();
                        if num_nodes != 0 {
                            self.update_bounding_envelope();
                        }

                        (true, num_nodes < m, deleted_entry)
                    }
                }
            }
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

fn find_closest_child<'a, P, K: Distance + DimIndex + PartialEq, const M: usize>(
    children: &'a [SsNode<P, K, M>],
    target: &K,
) -> &'a SsNode<P, K, M> {
    let mut min_dist = f32::MAX;
    let mut cur_min = None;
    for child in children {
        let d = child.center_radius.center.distance(target);
        if d < min_dist {
            min_dist = d;
            cur_min = Some(child);
        }
    }
    cur_min.unwrap()
}
fn find_closest_child_index<P, K: Distance + DimIndex + PartialEq, const M: usize>(
    children: &[SsNode<P, K, M>],
    target: &K,
) -> usize {
    let mut min_dist = f32::MAX;
    let mut cur_min = None;
    for (i, child) in children.iter().enumerate() {
        let d = child.center_radius.center.distance(target);
        if d < min_dist {
            min_dist = d;
            cur_min = Some(i);
        }
    }
    cur_min.unwrap()
}

#[derive(Debug)]
pub struct SsTree<P, K: Distance + DimIndex + PartialEq, const M: usize> {
    pub root: SsNode<P, K, M>,
    height: usize,
    m: usize,
}

impl<P, K: Default + Distance + DimIndex + PartialEq, const M: usize> SsTree<P, K, M> {
    pub fn new(m: usize) -> Self {
        Self {
            root: SsNode {
                center_radius: CenterRadius {
                    center: K::default(),
                    radius: 0f32,
                },
                links: SsNodeLinks::Leaf(Box::new(ArrayVec::new())),
            },
            height: 1,
            m,
        }
    }

    pub fn insert(&mut self, payload: P, center: K, radius: f32) {
        self.insert_entry(Entry {
            center_radius: CenterRadius { center, radius },
            payload,
        })
    }
    pub fn insert_entry(&mut self, entry: Entry<P, K>) {
        if let Some((new_child_1, new_child_2)) = self.root.insert(entry, self.m) {
            let mut nodes = ArrayVec::<_, M>::new();
            nodes.push(new_child_1);
            nodes.push(new_child_2);
            let (centroid, radius) = inner::centroid_and_radius(&nodes);
            self.root = SsNode {
                center_radius: CenterRadius {
                    center: centroid,
                    radius,
                },
                links: SsNodeLinks::Inner(Box::new(nodes)),
            };
            self.height += 1;
        }
    }

    #[allow(clippy::overly_complex_bool_expr)]
    pub fn remove(&mut self, point: &K) {
        let (_deleted, _violiates_invariant) = self.root.remove(point, self.m);

        match &mut self.root.links {
            SsNodeLinks::Inner(nodes) if nodes.len() == 1 => {
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
        let (num_points, num_nodes) = self.root.count_nodes();
        num_points as f32 / num_nodes as f32
    }

    pub fn find_entries_within_radius<'a>(
        &'a self,
        center: &K,
        radius: f32,
        out: &mut Vec<&'a Entry<P, K>>,
    ) {
        self.root.find_entries_within_radius(center, radius, out);
    }

    pub fn find_if<'a, F: Fn(&P) -> bool>(
        &'a self,
        center: &K,
        radius: f32,
        f: F,
    ) -> Option<&'a Entry<P, K>> {
        self.root.find_if(center, radius, &f)
    }
    pub fn remove_if<F: Fn(&P) -> bool>(
        &mut self,
        center: &K,
        radius: f32,
        f: F,
    ) -> Option<Entry<P, K>> {
        let deleted_entry = self.root.remove_if(center, radius, self.m, &f).2;
        match &mut self.root.links {
            SsNodeLinks::Inner(nodes) if nodes.len() == 1 => {
                self.root = nodes.pop().unwrap();
                self.height -= 1;
            }
            _ => (),
        }
        deleted_entry
    }
}

impl<P, K: Distance + DimIndex + Default + PartialEq, const M: usize> Default for SsTree<P, K, M> {
    fn default() -> Self {
        Self::new(M / 2)
    }
}

mod util {
    use super::{DimIndex, Distance, Entry, SsNode};

    pub trait GetCenter<K> {
        fn get_center(&self) -> &K;
    }

    impl<P, K> GetCenter<K> for Entry<P, K> {
        fn get_center(&self) -> &K {
            &self.center_radius.center
        }
    }

    impl<P, K: Distance + DimIndex + PartialEq, const M: usize> GetCenter<K> for SsNode<P, K, M> {
        fn get_center(&self) -> &K {
            &self.center_radius.center
        }
    }

    pub fn mean_along_direction<K: DimIndex, E: GetCenter<K>>(
        entry: &[E],
        direction_index: usize,
    ) -> f32 {
        assert!(!entry.is_empty());
        let count = entry.len() as f32;
        let sum = entry
            .iter()
            .map(|point| point.get_center()[direction_index])
            .sum::<f32>();
        sum / count
    }

    pub fn variance_along_direction<K: DimIndex, E: GetCenter<K>>(
        entries: &[E],
        direction_index: usize,
    ) -> f32 {
        assert!(!entries.is_empty());
        let mean = mean_along_direction(entries, direction_index);
        let count = entries.len() as f32;
        entries
            .iter()
            .map(|point| {
                let diff = mean - point.get_center()[direction_index];
                diff * diff
            })
            .sum::<f32>()
            / count
    }

    pub fn direction_of_max_variance<K: DimIndex, E: GetCenter<K>>(entries: &[E]) -> usize {
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

    pub fn centroid<K: DimIndex + Default, E: GetCenter<K>>(entries: &[E]) -> K {
        let mut centroid = K::default();
        for i in 0..K::NUM_DIMENSIONS {
            centroid[i] = mean_along_direction(entries, i);
        }
        centroid
    }
}

mod leaf {
    use super::{
        util::{centroid, direction_of_max_variance, variance_along_direction},
        DimIndex, Distance, Entry,
    };

    pub fn find_split_index<P, K: DimIndex + Distance, const M: usize>(
        entries: &mut [Entry<P, K>],
        m: usize,
    ) -> usize {
        let coordinate_index = direction_of_max_variance(entries);
        entries.sort_by(|p1, p2| {
            p1.center_radius.center[coordinate_index]
                .partial_cmp(&p2.center_radius.center[coordinate_index])
                .unwrap()
        });

        let mut min_variance = f32::INFINITY;
        let mut split_index = m;
        for i in m..=(entries.len() - m) {
            let variance1 = variance_along_direction(&entries[..i], coordinate_index);
            let variance2 = variance_along_direction(&entries[i..], coordinate_index);
            let variance = variance1 + variance2;
            if variance < min_variance {
                min_variance = variance;
                split_index = i;
            }
        }
        split_index
    }

    pub fn centroid_and_radius<P, K: Default + DimIndex + Distance, const M: usize>(
        entires: &[Entry<P, K>],
    ) -> (K, f32) {
        let centroid = centroid(entires);

        let radius = entires
            .iter()
            .map(|node| centroid.distance(&node.center_radius.center) + node.center_radius.radius)
            .max_by(|d1, d2| d1.partial_cmp(d2).unwrap())
            .unwrap();
        (centroid, radius)
    }
}
mod inner {
    use arrayvec::ArrayVec;

    use super::{
        util::{centroid, direction_of_max_variance, variance_along_direction},
        DimIndex, Distance, SsNode, SsNodeLinks,
    };

    pub fn centroid_and_radius<P, K: DimIndex + Distance + PartialEq + Default, const M: usize>(
        nodes: &[SsNode<P, K, M>],
    ) -> (K, f32) {
        let centroid = centroid(nodes);
        let radius = nodes
            .iter()
            .map(|node| centroid.distance(&node.center_radius.center) + node.center_radius.radius)
            .max_by(|d1, d2| d1.partial_cmp(d2).unwrap())
            .unwrap();
        (centroid, radius)
    }

    pub fn find_split_index<P, K: Distance + DimIndex + PartialEq, const M: usize>(
        nodes: &mut [SsNode<P, K, M>],
        m: usize,
    ) -> usize {
        let coordinate_index = direction_of_max_variance(nodes);
        nodes.sort_by(|p1, p2| {
            p1.center_radius.center[coordinate_index]
                .partial_cmp(&p2.center_radius.center[coordinate_index])
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

    pub fn find_sibling_to_borrow_from<P, K: Distance + DimIndex + PartialEq, const M: usize>(
        nodes: &[SsNode<P, K, M>],
        node_to_fix: usize,
        m: usize,
    ) -> Option<usize> {
        let siblings_to_borrow_from =
            nodes
                .iter()
                .enumerate()
                .filter(|(i, sibling)| match &sibling.links {
                    SsNodeLinks::Inner(nodes) => *i != node_to_fix && nodes.len() > m,
                    SsNodeLinks::Leaf(points) => *i != node_to_fix && points.len() > m,
                });

        let mut closest_sibling = None;
        let mut closest_sibling_dist = f32::INFINITY;

        for (i, sibling) in siblings_to_borrow_from {
            let distance = nodes[node_to_fix]
                .center_radius
                .center
                .distance(&sibling.center_radius.center);
            if distance < closest_sibling_dist {
                closest_sibling = Some(i);
                closest_sibling_dist = distance;
            }
        }
        closest_sibling
    }

    pub fn borrow_from_sibling<P, K: Distance + Default + DimIndex + PartialEq, const M: usize>(
        nodes: &mut [SsNode<P, K, M>],
        node_to_fix: usize,

        sibling_to_borrow_from: usize,
    ) {
        // found sibling to borrow from
        let to_fix_centroid = &nodes[node_to_fix].center_radius.center;
        match &mut nodes[sibling_to_borrow_from].links {
            SsNodeLinks::Inner(nodes2) => {
                let mut closest_node = None;
                let mut closest_node_dist = f32::INFINITY;
                for (i, node) in nodes2.iter().enumerate() {
                    let distance = node.center_radius.center.distance(to_fix_centroid);
                    if distance < closest_node_dist {
                        closest_node = Some(i);
                        closest_node_dist = distance;
                    }
                }
                let node = nodes2.remove(closest_node.unwrap());
                nodes[sibling_to_borrow_from].update_bounding_envelope();

                match &mut nodes[node_to_fix].links {
                    SsNodeLinks::Inner(fix_nodes) => fix_nodes.push(node),
                    SsNodeLinks::Leaf(_) => panic!("unbalanced tree"),
                }
                nodes[node_to_fix].update_bounding_envelope();
            }
            SsNodeLinks::Leaf(points) => {
                let mut closest_point = None;
                let mut closest_point_dist = f32::INFINITY;
                for (i, point) in points.iter().enumerate() {
                    let distance = point.center_radius.center.distance(to_fix_centroid);
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
                nodes[sibling_to_borrow_from].update_bounding_envelope();
                match &mut nodes[node_to_fix].links {
                    SsNodeLinks::Inner(_) => panic!("unbalanced tree"),
                    SsNodeLinks::Leaf(fix_points) => fix_points.push(point),
                }
                nodes[node_to_fix].update_bounding_envelope();
            }
        }
    }

    pub fn find_sibling_to_merge_to<P, K: Distance + DimIndex + PartialEq, const M: usize>(
        nodes: &[SsNode<P, K, M>],
        node_to_fix: usize,
        m: usize,
    ) -> Option<usize> {
        let siblings_to_merge_to =
            nodes
                .iter()
                .enumerate()
                .filter(|(i, sibling)| match &sibling.links {
                    SsNodeLinks::Inner(nodes) => *i != node_to_fix && nodes.len() == m,
                    SsNodeLinks::Leaf(points) => *i != node_to_fix && points.len() == m,
                });

        let mut closest_sibling = None;
        let mut closest_sibling_dist = f32::INFINITY;

        for (i, sibling) in siblings_to_merge_to {
            let distance = nodes[node_to_fix]
                .center_radius
                .center
                .distance(&sibling.center_radius.center);
            if distance < closest_sibling_dist {
                closest_sibling = Some(i);
                closest_sibling_dist = distance;
            }
        }
        closest_sibling
    }

    pub fn merge_siblings<P, K: Default + Distance + DimIndex + PartialEq, const M: usize>(
        nodes: &mut ArrayVec<SsNode<P, K, M>, M>,
        mut node_index_1: usize,
        mut node_index_2: usize,
    ) {
        if node_index_1 > node_index_2 {
            // remove node with larger index first
            std::mem::swap(&mut node_index_1, &mut node_index_2);
        }
        let node_2 = nodes.remove(node_index_2);
        let node_1 = nodes.remove(node_index_1);
        let node = merge(node_1, node_2);
        nodes.push(node);
    }

    fn merge<P, K: Default + Distance + DimIndex + PartialEq, const M: usize>(
        node_1: SsNode<P, K, M>,
        node_2: SsNode<P, K, M>,
    ) -> SsNode<P, K, M> {
        match (node_1.links, node_2.links) {
            (SsNodeLinks::Leaf(mut points1), SsNodeLinks::Leaf(mut points2)) => {
                points1.extend(points2.drain(..));
                SsNode::<P, K, M>::from_entries(*points1)
            }
            (SsNodeLinks::Inner(mut nodes1), SsNodeLinks::Inner(mut nodes2)) => {
                nodes1.extend(nodes2.drain(..));
                SsNode::<P, K, M>::from_nodes(*nodes1)
            }
            _ => panic!("inconsistent siblings"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Entry;
    use super::SsTree;

    impl<P, K: PartialEq> PartialEq for Entry<P, K> {
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

        tree.insert_entry(Entry::new([0.0, 0.0], 1.0, ()));
        tree.insert_entry(Entry::new([5.0, 5.0], 1.0, ()));

        let mut out = Vec::new();
        tree.find_entries_within_radius(&[0.5, 0.5], 1.0, &mut out);
        assert_eq!(out, vec!(&Entry::new([0.0, 0.0], 1.0, ())));

        let mut out = Vec::new();
        tree.find_entries_within_radius(&[4.5, 5.5], 1.0, &mut out);
        assert_eq!(out, vec!(&Entry::new([5.0, 5.0], 1.0, ())));
        let mut out = Vec::new();

        // do search between the entries with radius big enough to just reach them
        tree.find_entries_within_radius(
            &[2.5, 2.5],
            (2.5 * std::f32::consts::SQRT_2 + 0.0001) - 1.0,
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert!(out.contains(&&Entry::new([5.0, 5.0], 1.0, ())));
        assert!(out.contains(&&Entry::new([0.0, 0.0], 1.0, ())));

        let mut out = Vec::new();

        // the same as befor but with radius just barely too small
        tree.find_entries_within_radius(
            &[2.5, 2.5],
            (2.5 * std::f32::consts::SQRT_2 - 0.0001) - 1.0,
            &mut out,
        );
        assert!(out.is_empty());
    }
}

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, bevy_inspector_egui::Inspectable)]
pub struct SpatialBounds {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Resource, Default)]
pub struct SpatialIndex {
    sstree: SsTree<Entity, Vec3, 8>,
}

impl SpatialIndex {
    pub fn clear(&mut self) {
        self.sstree = SsTree::default();
    }
    pub fn update(&mut self, entity: Entity, from: Option<SpatialBounds>, to: SpatialBounds) {
        if let Some(SpatialBounds { center, radius }) = from {
            if self
                .sstree
                .remove_if(&center, radius, |e| *e == entity)
                .is_none()
            {
                error!("failed to remove brush from spatial index for update");
                panic!("aborting");
            }
        }
        self.sstree.insert(entity, to.center, to.radius);
    }

    pub fn remove(&mut self, entity: Entity, bounds: SpatialBounds) {
        self.sstree
            .remove_if(&bounds.center, bounds.radius, |e| *e == entity);
    }

    pub fn query(&self, bounds: SpatialBounds) -> impl Iterator<Item = Entity> + '_ {
        let mut out = Vec::new();
        self.sstree
            .find_entries_within_radius(&bounds.center, bounds.radius, &mut out);
        out.into_iter().map(|e| e.payload)
    }
}
