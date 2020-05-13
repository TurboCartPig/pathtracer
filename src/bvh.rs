use crate::{
    primitives::{Instance, AABB},
    Hit, Intersect, Ray,
};
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

trait GetAxis {
    type Output;

    fn axis(&self, axis: Axis) -> Self::Output;
}

impl GetAxis for Vec3 {
    type Output = f32;

    fn axis(&self, axis: Axis) -> Self::Output {
        match axis {
            Axis::X => self.x(),
            Axis::Y => self.y(),
            Axis::Z => self.z(),
        }
    }
}

/// A Bounding Volume Hirarchy
pub struct BVH {
    /// The primitives that make up the scene
    geometry: Vec<Instance>,
    /// The BVH tree
    tree: Vec<FlatNode>,
}

impl BVH {
    pub fn new(geometry: Vec<Instance>) -> Self {
        assert!(!geometry.is_empty());

        // How many primitives can be in the same node
        let split_threshold = 64;
        let mut total_nodes = 0;
        // Convert from node index to geometry index, and use this to sort the geometry later
        // The indices are as seen from the nodes and the elements from the geometry
        let mut index_to_geometry = Vec::new();
        // Precompute build info about the geometry
        let mut build_geomentry = geometry
            .iter()
            .enumerate()
            .map(|(index, geom)| {
                let bounds = geom.bounds().unwrap();
                let center = 0.5 * (bounds.max - bounds.min);
                GeometryInfo {
                    geom,
                    index,
                    center,
                    bounds,
                }
            })
            .collect::<Vec<_>>();

        let root = BVH::build(
            &mut build_geomentry,
            &mut index_to_geometry,
            &mut total_nodes,
            split_threshold,
        );

        // Make a flat tree of FlatNodes from the root node of a BuildNode tree
        let tree = Self::flatten(root, total_nodes);

        // Sort the geometry by the indices in index_to_geometry
        let geometry = index_to_geometry
            .into_iter()
            .map(|i| geometry.get(i).unwrap())
            .cloned()
            .collect();

        // geometry.sort_unstable_by_key(|_| index_to_geometry.get(index???));

        println!("Total Nodes Built: {}", total_nodes);

        Self { geometry, tree }
    }

    fn build(
        geometry: &mut [GeometryInfo],
        index_to_geometry: &mut Vec<usize>,
        total_nodes: &mut usize,
        split_threshold: usize,
    ) -> BuildNode {
        *total_nodes += 1;

        // Create bounding box for all geometry in this BuildNode
        let bounds = geometry
            .iter()
            .fold(AABB::default(), |b, g| b.union(g.geom.bounds().unwrap()));

        // Check if we are a leaf
        if geometry.len() == 1 {
            return BVH::build_leaf(geometry, index_to_geometry, bounds);
        }

        // Create centroids for all geometry in this BuildNode
        let centroids = geometry
            .iter()
            .fold(AABB::default(), |b, g| b.point_union(g.center));

        // Decide which axis to spilt the scene along
        let split_axis = bounds.max_extent();
        let mid;

        // SAH guided partitioning
        let mut buckets = [SAHBucket::default(); 12];
        for g in geometry.iter() {
            let b = ((g.center.axis(split_axis) - centroids.min.axis(split_axis))
                / (centroids.max.axis(split_axis) - centroids.min.axis(split_axis))
                * buckets.len() as f32) as usize;
            let b = if b == buckets.len() { b - 1 } else { b };
            if let Some(bucket) = buckets.get_mut(b) {
                bucket.count += 1;
                bucket.bounds = bucket.bounds.union(g.bounds);
            }
        }

        let mut cost = [0.0; 11];
        for (i, c) in cost.iter_mut().enumerate() {
            let left = buckets
                .iter()
                .take(i + 1)
                .fold(SAHBucket::default(), |mut a, b| {
                    a.bounds = a.bounds.union(b.bounds);
                    a.count += b.count;
                    a
                });
            let right = buckets
                .iter()
                .skip(i + 1)
                .fold(SAHBucket::default(), |mut a, b| {
                    a.bounds = a.bounds.union(b.bounds);
                    a.count += b.count;
                    a
                });

            *c = 0.125
                + (left.count as f32 * left.bounds.surface_area()
                    + right.count as f32 * right.bounds.surface_area())
                    / bounds.surface_area();
        }

        let (min_bucket, min_cost) =
            cost.iter()
                .enumerate()
                .fold((0, std::f32::INFINITY), |(pi, pc), (i, c)| {
                    if *c < pc {
                        (i, *c)
                    } else {
                        (pi, pc)
                    }
                });

        // Check if we should build an interior node based on cost and the split_threshold
        if geometry.len() > split_threshold || min_cost < geometry.len() as f32 {
            // Partition the geometry into a half that fails the predicate, and a half that
            // satisfies it. Then return the index of the first element to satisfie the predicate

            let func = |g: &GeometryInfo| {
                let b = ((g.center.axis(split_axis) - centroids.min.axis(split_axis))
                    / (centroids.max.axis(split_axis) - centroids.min.axis(split_axis))
                    * buckets.len() as f32) as usize;
                let b = if b == buckets.len() { b - 1 } else { b };
                b <= min_bucket
            };

            geometry.sort_unstable_by_key(func);
            mid = geometry.iter().position(func).unwrap_or(geometry.len() / 2);
        } else {
            return BVH::build_leaf(geometry, index_to_geometry, bounds);
        }

        println!("Geometry count: {}\nMid: {}", geometry.len(), mid);

        // Assert that mid can be used to make valid ranges
        assert!(mid != 0 && mid != geometry.len());
        let left = Box::new(BVH::build(
            &mut geometry[..mid],
            index_to_geometry,
            total_nodes,
            split_threshold,
        ));
        let right = Box::new(BVH::build(
            &mut geometry[mid..],
            index_to_geometry,
            total_nodes,
            split_threshold,
        ));
        BuildNode::interior(left, right)
    }

    fn build_leaf(
        geometry: &mut [GeometryInfo],
        index_to_geometry: &mut Vec<usize>,
        bounds: AABB,
    ) -> BuildNode {
        let geometry_offset = index_to_geometry.len();
        index_to_geometry.append(&mut geometry.iter().map(|g| g.index).collect());
        BuildNode::leaf(bounds, geometry_offset, geometry.len())
    }

    fn flatten(root: BuildNode, size: usize) -> Vec<FlatNode> {
        let mut tree = Vec::with_capacity(size);
        Self::flatten_impl(root, &mut tree);

        tree
    }

    fn flatten_impl(node: BuildNode, tree: &mut Vec<FlatNode>) -> usize {
        let offset = tree.len();
        match node.inner {
            BuildNodeInner::Interior { left, right } => {
                tree.push(FlatNode::interior(node.bounds, 0, 0));
                let left_idx = Self::flatten_impl(*left, tree);
                let right_idx = Self::flatten_impl(*right, tree);
                match tree[offset].inner {
                    FlatNodeInner::Interior {
                        ref mut left,
                        ref mut right,
                        ..
                    } => {
                        *left = left_idx;
                        *right = right_idx;
                    }
                    _ => panic!("Node changed while initializing it?!"),
                }
            }
            BuildNodeInner::Leaf {
                geometry_offset,
                num_primitives,
            } => {
                tree.push(FlatNode::leaf(node.bounds, geometry_offset, num_primitives));
            }
        }

        offset
    }
}

impl Intersect for BVH {
    fn intersection(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<Hit> {
        fn intersect(
            node: &FlatNode,
            tree: &[FlatNode],
            geometry: &[impl Intersect],
            ray: Ray,
            t_min: f32,
            t_max: f32,
        ) -> Option<Hit> {
            if node.bounds.has_intersection(ray, t_min, t_max) {
                match node.inner {
                    FlatNodeInner::Interior { left, right, .. } => {
                        let left = tree
                            .get(left)
                            .and_then(|node| intersect(node, tree, geometry, ray, t_min, t_max));
                        let right = tree
                            .get(right)
                            .and_then(|node| intersect(node, tree, geometry, ray, t_min, t_max));

                        return match (left, right) {
                            (Some(left), Some(right)) => {
                                if left.t < right.t {
                                    Some(left)
                                } else {
                                    Some(right)
                                }
                            }
                            (left, None) => left,
                            (None, right) => right,
                        };

                        // if let Some(left) = left {
                        //     if let Some(right) = right {
                        //         // Compare who is closest
                        //         if left.t < right.t {
                        //             return Some(left);
                        //         } else {
                        //             return Some(right);
                        //         }
                        //     } else {
                        //         return Some(left);
                        //     }
                        // } else if right.is_some() {
                        //     return right;
                        // }
                    }
                    FlatNodeInner::Leaf {
                        geometry_offset,
                        num_primitives,
                    } => {
                        let mut hit = None;
                        let mut closest = t_max;

                        // Find the closest intersection
                        for primitive in
                            &geometry[geometry_offset..geometry_offset + num_primitives]
                        {
                            if let Some(h) = primitive.intersection(ray, t_min, closest) {
                                closest = h.t;
                                hit = Some(h);
                            }
                        }

                        return hit;
                    }
                }
            }

            None
        };

        let node = self.tree.first().unwrap();
        intersect(node, &self.tree, &self.geometry, ray, t_min, t_max)
    }

    fn has_intersection(&self, _ray: Ray, _t_min: f32, _t_max: f32) -> bool {
        unimplemented!()
    }

    fn bounds(&self) -> Option<AABB> {
        self.tree.first().map(|node| node.bounds)
    }
}

struct GeometryInfo<'a> {
    geom: &'a Instance,
    index: usize,
    center: Vec3,
    bounds: AABB,
}

#[derive(Copy, Clone, Debug, Default)]
struct SAHBucket {
    count: usize,
    bounds: AABB,
}

#[derive(Debug)]
enum BuildNodeInner {
    Interior {
        left: Box<BuildNode>,
        right: Box<BuildNode>,
    },
    Leaf {
        geometry_offset: usize,
        num_primitives: usize,
    },
}

#[derive(Debug)]
struct BuildNode {
    bounds: AABB,
    inner: BuildNodeInner,
}

impl BuildNode {
    fn interior(left: Box<BuildNode>, right: Box<BuildNode>) -> Self {
        let bounds = left.bounds.union(right.bounds);

        Self {
            bounds,
            inner: BuildNodeInner::Interior { left, right },
        }
    }

    fn leaf(bounds: AABB, geometry_offset: usize, num_primitives: usize) -> Self {
        Self {
            bounds,
            inner: BuildNodeInner::Leaf {
                geometry_offset,
                num_primitives,
            },
        }
    }
}

#[derive(Debug)]
enum FlatNodeInner {
    Interior {
        left: usize,
        right: usize,
    },
    Leaf {
        geometry_offset: usize,
        num_primitives: usize,
    },
}

#[derive(Debug)]
struct FlatNode {
    bounds: AABB,
    inner: FlatNodeInner,
}

impl FlatNode {
    fn interior(bounds: AABB, left: usize, right: usize) -> Self {
        Self {
            bounds,
            inner: FlatNodeInner::Interior { left, right },
        }
    }

    fn leaf(bounds: AABB, geometry_offset: usize, num_primitives: usize) -> Self {
        Self {
            bounds,
            inner: FlatNodeInner::Leaf {
                geometry_offset,
                num_primitives,
            },
        }
    }
}
