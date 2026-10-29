use bevy::{
    math::{Mat3A, Vec3A},
    prelude::*
};

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3
}
impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}
impl Aabb {
    /// Contains nothing, and is the identity of [`Self::union`].
    pub const EMPTY: Self = Self {
        min: Vec3::INFINITY,
        max: Vec3::NEG_INFINITY
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn is_empty(&self) -> bool {
        self.min.cmpgt(self.max).any()
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max)
        }
    }

    pub fn grow(self, margin: Vec3) -> Self {
        Self {
            min: self.min - margin,
            max: self.max + margin
        }
    }

    /// Bounds of this box once transformed by `transform`.
    pub fn transformed(&self, transform: &GlobalTransform) -> Self {
        if self.is_empty() {
            return *self;
        }
        let affine = transform.affine();
        let center = affine.transform_point3a(Vec3A::from(self.center()));
        let m = affine.matrix3;
        let abs = Mat3A::from_cols(m.x_axis.abs(), m.y_axis.abs(), m.z_axis.abs());
        let half = abs * Vec3A::from(self.half_extents());
        Self {
            min: (center - half).into(),
            max: (center + half).into()
        }
    }
}

/// Where a box lies relative to a [`Frustum`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrustumTest {
    Outside,
    Intersect,
    Inside
}

/// The 6 planes of a view volume, normals pointing inside.
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    planes: [Vec4; 6]
}
impl Frustum {
    /// Frustum of a world to clip space matrix, with depth in `[0, 1]` (wgpu convention).
    pub fn from_view_proj(view_proj: Mat4) -> Self {
        let [r0, r1, r2, r3] = [0, 1, 2, 3].map(|i| view_proj.row(i));
        let planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2].map(|p| {
            let length = p.xyz().length();
            if length > 0.0 { p / length } else { p }
        });
        Self { planes }
    }

    pub fn test(&self, aabb: &Aabb) -> FrustumTest {
        let (center, half) = (aabb.center(), aabb.half_extents());
        let mut inside = true;
        for plane in &self.planes {
            let distance = plane.xyz().dot(center) + plane.w;
            let radius = plane.xyz().abs().dot(half);
            if distance + radius < 0.0 {
                return FrustumTest::Outside;
            }
            inside &= distance - radius >= 0.0;
        }
        match inside {
            true => FrustumTest::Inside,
            false => FrustumTest::Intersect
        }
    }

    pub fn intersects(&self, aabb: &Aabb) -> bool {
        let (center, half) = (aabb.center(), aabb.half_extents());
        self.planes
            .iter()
            .all(|plane| plane.xyz().dot(center) + plane.w + plane.xyz().abs().dot(half) >= 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::CameraView;

    fn camera_frustum() -> Frustum {
        // At the origin, looking down -Z
        let view = CameraView::default();
        Frustum::from_view_proj(view.world_to_ndc(&GlobalTransform::IDENTITY, 16.0 / 9.0))
    }

    fn cube(center: Vec3, half: f32) -> Aabb {
        Aabb::new(center - half, center + half)
    }

    #[test]
    fn perspective_classification() {
        let frustum = camera_frustum();
        assert_eq!(
            frustum.test(&cube(Vec3::new(0.0, 0.0, -10.0), 1.0)),
            FrustumTest::Inside
        );
        assert_eq!(
            frustum.test(&cube(Vec3::new(0.0, 0.0, 10.0), 1.0)),
            FrustumTest::Outside
        );
        assert_eq!(
            frustum.test(&cube(Vec3::new(100.0, 0.0, -10.0), 1.0)),
            FrustumTest::Outside
        );
        assert_eq!(frustum.test(&cube(Vec3::ZERO, 1.0)), FrustumTest::Intersect);
        assert!(!frustum.intersects(&cube(Vec3::new(0.0, 0.0, -20_000.0), 1.0)));
    }

    #[test]
    fn intersects_matches_test() {
        let frustum = camera_frustum();
        for x in -20..=20 {
            for z in -20..=20 {
                let aabb = cube(Vec3::new(x as f32 * 3.0, 0.0, z as f32 * 3.0), 1.0);
                assert_eq!(
                    frustum.intersects(&aabb),
                    frustum.test(&aabb) != FrustumTest::Outside
                );
            }
        }
    }

    #[test]
    fn orthographic_depth_range() {
        // Depth mapped to [0, 1] between 0 and 10 units in front of the eye
        let proj = Mat4::orthographic_rh(-5.0, 5.0, -5.0, 5.0, 0.0, 10.0);
        let frustum = Frustum::from_view_proj(proj);
        assert!(frustum.intersects(&cube(Vec3::new(0.0, 0.0, -5.0), 1.0)));
        assert!(!frustum.intersects(&cube(Vec3::new(0.0, 0.0, 5.0), 1.0)));
        assert!(!frustum.intersects(&cube(Vec3::new(0.0, 0.0, -15.0), 1.0)));
    }

    #[test]
    fn transformed_bounds() {
        let aabb = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0));
        let transform = GlobalTransform::from(
            Transform::from_xyz(10.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(2.0))
        );
        let moved = aabb.transformed(&transform);
        assert!(moved.min.abs_diff_eq(Vec3::new(10.0, 0.0, -4.0), 1e-4));
        assert!(moved.max.abs_diff_eq(Vec3::new(12.0, 2.0, 0.0), 1e-4));
        assert!(Aabb::EMPTY.transformed(&transform).is_empty());
        assert_eq!(Aabb::EMPTY.union(aabb), aabb);
    }
}
