use std::f64::consts::PI;

use crate::{
    Anchor, Component, Node, NodeType, Path,
    font::{NodeAttrs, Scale},
};

impl From<&norad::Contour> for Path {
    fn from(contour: &norad::Contour) -> Self {
        let mut nodes: Vec<Node> = contour
            .points
            .iter()
            .map(|contour| contour.into())
            .collect();

        if contour.is_closed() && !nodes.is_empty() {
            // In Glyphs.app, the starting node of a closed contour is
            // always stored at the end of the nodes list.
            nodes.rotate_left(1);
        }

        Self {
            attr: None,
            closed: contour.is_closed(),
            nodes,
        }
    }
}

impl TryFrom<&Path> for norad::Contour {
    type Error = norad::error::NamingError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let mut points = path
            .nodes
            .iter()
            .map(|node| node.try_into())
            .collect::<Result<Vec<norad::ContourPoint>, _>>()?;

        if !points.is_empty() {
            if !path.closed {
                // This logic comes from glyphsLib.
                assert!(points[0].typ == norad::PointType::Line);
                points[0].typ = norad::PointType::Move;
            } else {
                // In Glyphs.app, the starting node of a closed contour is
                // always stored at the end of the nodes list.
                points.rotate_right(1);
            }
        }
        Ok(Self::new(points, None))
    }
}

impl From<&norad::ContourPoint> for Node {
    fn from(point: &norad::ContourPoint) -> Self {
        Self {
            pt: kurbo::Point::new(point.x, point.y),
            node_type: match (&point.typ, point.smooth) {
                (norad::PointType::Move, _) => NodeType::Line,
                (norad::PointType::Line, true) => NodeType::LineSmooth,
                (norad::PointType::Line, false) => NodeType::Line,
                (norad::PointType::OffCurve, _) => NodeType::OffCurve,
                (norad::PointType::Curve, true) => NodeType::CurveSmooth,
                (norad::PointType::Curve, false) => NodeType::Curve,
                (norad::PointType::QCurve, true) => NodeType::QCurveSmooth,
                (norad::PointType::QCurve, false) => NodeType::QCurve,
            },
            attr: point.name.as_ref().map(|name| NodeAttrs {
                name: Some(name.to_string()),
                other_stuff: Default::default(),
            }),
        }
    }
}

impl TryFrom<&Node> for norad::ContourPoint {
    type Error = norad::error::NamingError;

    fn try_from(node: &Node) -> Result<Self, Self::Error> {
        let (typ, smooth) = match &node.node_type {
            NodeType::Curve => (norad::PointType::Curve, false),
            NodeType::CurveSmooth => (norad::PointType::Curve, true),
            NodeType::Line => (norad::PointType::Line, false),
            NodeType::LineSmooth => (norad::PointType::Line, true),
            NodeType::OffCurve => (norad::PointType::OffCurve, false),
            NodeType::QCurve => (norad::PointType::QCurve, false),
            NodeType::QCurveSmooth => (norad::PointType::QCurve, true),
        };
        Ok(Self::new(
            node.pt.x,
            node.pt.y,
            typ,
            smooth,
            node.attr
                .as_ref()
                .and_then(|attr| attr.name.as_deref())
                .map(norad::Name::new)
                .transpose()?,
            None,
        ))
    }
}

impl From<&norad::Component> for Component {
    fn from(component: &norad::Component) -> Self {
        let (rotation, slant, scale, pos) = if component.transform == Default::default() {
            (None, None, None, None)
        } else {
            let (s_x, s_y, r) = transform_struct_to_scale_and_rotation(&component.transform);
            (
                Some(r),
                None,
                Some(Scale {
                    horizontal: s_x,
                    vertical: s_y,
                }),
                Some(kurbo::Point::new(
                    component.transform.x_offset,
                    component.transform.y_offset,
                )),
            )
        };
        Self {
            reference: component.base.to_string(),
            rotation,
            pos,
            scale,
            slant,
            other_stuff: Default::default(),
        }
    }
}

fn transform_struct_to_scale_and_rotation(transform: &norad::AffineTransform) -> (f64, f64, f64) {
    let det = transform.x_scale * transform.y_scale - transform.xy_scale * transform.yx_scale;
    let mut s_x = (transform.x_scale.powi(2) + transform.xy_scale.powi(2)).sqrt();
    let mut s_y = (transform.yx_scale.powi(2) + transform.y_scale.powi(2)).sqrt();

    if det < 0.0 {
        s_y = -s_y;
    }

    let mut r = (transform.xy_scale * s_y).atan2(transform.x_scale * s_x) * 180.0 / PI;

    if det < 0.0 && (r.abs() > 135.0 || r < -90.0) {
        s_x = -s_x;
        s_y = -s_y;
        if r < 0.0 {
            r += 180.0;
        } else {
            r -= 180.0;
        }
    }

    let mut quadrant = 0.0;
    if r < -90.0 {
        quadrant = 180.0;
        r += quadrant;
    }
    if r > 90.0 {
        quadrant = -180.0;
        r += quadrant;
    }

    r = r * s_x / s_y;
    r -= quadrant;
    if r < -179.0 {
        r += 360.0;
    }

    (s_x, s_y, r)
}

impl TryFrom<&Component> for norad::Component {
    type Error = norad::error::NamingError;

    fn try_from(component: &Component) -> Result<Self, Self::Error> {
        let name = norad::Name::new(&component.reference)?;

        let offset_x = component.pos.map(|p| p.x).unwrap_or(0.0);
        let offset_y = component.pos.map(|p| p.y).unwrap_or(0.0);
        let rotation = component.rotation.unwrap_or(0.0).to_radians();
        let scale_x = component
            .scale
            .as_ref()
            .map(|s| s.horizontal)
            .unwrap_or(1.0);
        let scale_y = component.scale.as_ref().map(|s| s.vertical).unwrap_or(1.0);
        let skew_x = component
            .slant
            .as_ref()
            .map(|p| p.horizontal)
            .unwrap_or(0.0);
        let skew_y = component.slant.as_ref().map(|p| p.vertical).unwrap_or(0.0);

        // Warning: Don't use kurbo's .then_* methods because they apply the ops
        // in the wrong order! This matches the order glyphsLib does it in.
        let transform = kurbo::Affine::translate(kurbo::Vec2::new(offset_x, offset_y))
            * kurbo::Affine::rotate(rotation)
            * kurbo::Affine::scale_non_uniform(scale_x, scale_y)
            * kurbo::Affine::skew(skew_x, skew_y);

        // Round values for roundtrip testing.
        let transform = norad::AffineTransform {
            x_scale: f64_precision(transform.as_coeffs()[0], 5),
            xy_scale: f64_precision(transform.as_coeffs()[1], 5),
            yx_scale: f64_precision(transform.as_coeffs()[2], 5),
            y_scale: f64_precision(transform.as_coeffs()[3], 5),
            x_offset: f64_precision(transform.as_coeffs()[4], 5),
            y_offset: f64_precision(transform.as_coeffs()[5], 5),
        };

        Ok(Self::new(name, transform, None))
    }
}

fn f64_precision(v: f64, precision: i32) -> f64 {
    let r = 10f64.powi(precision);
    (v * r).round() / r
}

impl From<&norad::Anchor> for Anchor {
    fn from(anchor: &norad::Anchor) -> Self {
        Self {
            name: anchor.name.as_ref().unwrap().as_str().to_string(),
            orientation: None,
            pos: kurbo::Point::new(anchor.x, anchor.y),
            user_data: Default::default(),
        }
    }
}

impl TryFrom<&Anchor> for norad::Anchor {
    type Error = norad::error::NamingError;

    fn try_from(anchor: &Anchor) -> Result<Self, Self::Error> {
        let name = norad::Name::new(&anchor.name)?;
        Ok(Self::new(
            anchor.pos.x,
            anchor.pos.y,
            Some(name),
            None,
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    #[test]
    fn roundtrip_component_example() {
        let transform = norad::AffineTransform {
            x_scale: -1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: -1.0,
            x_offset: 250.0,
            y_offset: 657.0,
        };
        roundtrip_component(transform);
    }

    /// Test that shear gets lost in translation. This is unwanted, but is due
    /// to the reference Python code in glyphsLib not extracting it.
    #[test]
    #[should_panic]
    fn roundtrip_component_shear() {
        let transform = norad::AffineTransform {
            x_scale: 0.5,
            xy_scale: 0.0,
            yx_scale: 1.5,
            y_scale: 0.7,
            x_offset: 0.0,
            y_offset: 0.0,
        };
        roundtrip_component(transform);
    }

    proptest! {
        #[test]
        fn roundtrip_components(
            x_scale in -10000.0..10000.0,
            y_scale in -10000.0..10000.0,
            x_offset in -10000.0..10000.0,
            y_offset in -10000.0..10000.0,
        ) {
            let transform = norad::AffineTransform {
                x_scale,
                xy_scale: 0.0, // Also proptest once shear is extracted.
                yx_scale: 0.0, // Also proptest once shear is extracted.
                y_scale,
                x_offset,
                y_offset,
            };

            roundtrip_component(transform);
        }
    }

    fn roundtrip_component(transform: norad::AffineTransform) {
        let name = norad::Name::new("comma").unwrap();
        let norad_component1 = norad::Component::new(name, transform, None);
        let glyphs_component: crate::Component = (&norad_component1).into();
        let norad_component2: norad::Component = (&glyphs_component).try_into().unwrap();

        let t1 = norad_component1.transform;
        let t2 = norad_component2.transform;
        assert!(
            approx_equal(t1.x_scale, t2.x_scale, 0.00001),
            "x_scale differs: {t1:?} vs. {t2:?}",
        );
        assert!(
            approx_equal(t1.xy_scale, t2.xy_scale, 0.00001),
            "xy_scale differs: {t1:?} vs. {t2:?}",
        );
        assert!(
            approx_equal(t1.yx_scale, t2.yx_scale, 0.00001),
            "yx_scale differs: {t1:?} vs. {t2:?}",
        );
        assert!(
            approx_equal(t1.y_scale, t2.y_scale, 0.00001),
            "y_scale differs: {t1:?} vs. {t2:?}",
        );
        assert!(
            approx_equal(t1.x_offset, t2.x_offset, 0.00001),
            "x_offset differs: {t1:?} vs. {t2:?}",
        );
        assert!(
            approx_equal(t1.y_offset, t2.y_offset, 0.00001),
            "y_offset differs: {t1:?} vs. {t2:?}",
        );
    }

    fn approx_equal(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() < tolerance
    }

    #[test]
    fn roundtrip_point_name() {
        // Create a point with name 'hello world'.
        let point = norad::ContourPoint::new(
            0.0,
            0.0,
            norad::PointType::Move,
            false,
            Some(norad::Name::new("hello world").unwrap()),
            None,
        );

        // Round-trip it.
        let node = crate::Node::from(&point);
        let point_again = norad::ContourPoint::try_from(&node).unwrap();

        // Confirm that the name is unchanged.
        assert_eq!(point.name, point_again.name);
    }
}
