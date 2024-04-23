use std::f64::consts::PI;

use crate::{font::Scale, Anchor, Component, Node, NodeType, Path};

impl From<&norad::Contour> for Path {
    fn from(contour: &norad::Contour) -> Self {
        let mut nodes: Vec<Node> = contour
            .points
            .iter()
            .map(|contour| contour.into())
            .collect();
        if contour.is_closed() {
            // In Glyphs.app, the starting node of a closed contour is
            // always stored at the end of the nodes list.
            nodes.rotate_left(1);
        }
        Self {
            closed: contour.is_closed(),
            nodes,
        }
    }
}

impl From<&Path> for norad::Contour {
    fn from(path: &Path) -> Self {
        let mut points: Vec<norad::ContourPoint> =
            path.nodes.iter().map(|node| node.into()).collect();
        if !path.closed {
            // This logic comes from glyphsLib.
            assert!(points[0].typ == norad::PointType::Line);
            points[0].typ = norad::PointType::Move;
        } else {
            // In Glyphs.app, the starting node of a closed contour is
            // always stored at the end of the nodes list.
            points.rotate_right(1);
        }
        Self::new(points, None, None)
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
        }
    }
}

impl From<&Node> for norad::ContourPoint {
    fn from(node: &Node) -> Self {
        let (typ, smooth) = match &node.node_type {
            NodeType::Curve => (norad::PointType::Curve, false),
            NodeType::CurveSmooth => (norad::PointType::Curve, true),
            NodeType::Line => (norad::PointType::Line, false),
            NodeType::LineSmooth => (norad::PointType::Line, true),
            NodeType::OffCurve => (norad::PointType::OffCurve, false),
            NodeType::QCurve => (norad::PointType::QCurve, false),
            NodeType::QCurveSmooth => (norad::PointType::QCurve, true),
        };
        Self::new(node.pt.x, node.pt.y, typ, smooth, None, None, None)
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
        let rotation = component.rotation.map(|r| r.to_radians()).unwrap_or(0.0);
        let scale_x = component
            .scale
            .as_ref()
            .map(|s| s.horizontal)
            .unwrap_or(0.0);
        let scale_y = component.scale.as_ref().map(|s| s.vertical).unwrap_or(0.0);
        let skew_x = component
            .slant
            .as_ref()
            .map(|p| p.horizontal)
            .unwrap_or(0.0);
        let skew_y = component.slant.as_ref().map(|p| p.vertical).unwrap_or(0.0);
        let transform = (kurbo::Affine::translate(kurbo::Vec2::new(offset_x, offset_y))
            .then_rotate(rotation)
            .then_scale_non_uniform(scale_x, scale_y)
            * kurbo::Affine::skew(skew_x, skew_y))
        .into();

        Ok(Self::new(name, transform, None, None))
    }
}

impl From<&norad::Anchor> for Anchor {
    fn from(anchor: &norad::Anchor) -> Self {
        Self {
            name: anchor.name.as_ref().unwrap().as_str().to_string(),
            orientation: None,
            pos: Some(kurbo::Point::new(anchor.x, anchor.y)),
        }
    }
}

impl TryFrom<&Anchor> for norad::Anchor {
    type Error = norad::error::NamingError;

    fn try_from(anchor: &Anchor) -> Result<Self, Self::Error> {
        let name = norad::Name::new(&anchor.name)?;
        Ok(Self::new(
            anchor.pos.map(|p| p.x).unwrap_or(0.0),
            anchor.pos.map(|p| p.y).unwrap_or(0.0),
            Some(name),
            None,
            None,
            None,
        ))
    }
}
