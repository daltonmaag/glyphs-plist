use glyphs_plist::FromPlist;
use glyphs_plist::Plist;

#[test]
fn reads_metric_keys() {
    let font = glyphs_plist::Font::load("testdata/MetricsKeys.glyphs").unwrap();

    let ef = font
        .glyphs
        .iter()
        .find(|g| g.glyphname.as_str() == "ef")
        .expect("glyph to be tested wasn't found");
    assert_eq!(ef.left_metrics_key.as_deref(), Some("o"));
    assert_eq!(ef.right_metrics_key.as_deref(), Some("o"));
    assert_eq!(ef.width_metrics_key, None);

    let alphatonos = font
        .glyphs
        .iter()
        .find(|g| g.glyphname.as_str() == "Alphatonos")
        .expect("glyph to be tested wasn't found");
    assert_eq!(alphatonos.left_metrics_key, None);
    assert_eq!(alphatonos.right_metrics_key.as_deref(), Some("A"));
    assert_eq!(alphatonos.width_metrics_key.as_deref(), Some("A"));
}

#[test]
fn open_contour_smooth_point() {
    // Some glyphs files have open contours with smooth line points. This was
    // erroneously asserted never to occur before.
    let path_source = r#"
        {
            closed = 0;
            nodes = (
                "303 128 LINE SMOOTH"
            );
        }
    "#;

    let plist = Plist::parse(path_source).unwrap();
    let path: glyphs_plist::Path = glyphs_plist::Path::from_plist(plist);
    let contour: norad::Contour = (&path).into();

    assert!(!contour.is_closed());
    assert_eq!(
        contour.points,
        vec![norad::ContourPoint::new(
            303.0,
            128.0,
            norad::PointType::Move,
            true,
            None,
            None,
            None
        )]
    );
}
