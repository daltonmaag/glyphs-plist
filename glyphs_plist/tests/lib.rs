use glyphs_plist::Plist;

#[test]
fn reads_metric_keys() {
    let font = glyphs_plist::Font::load("testdata/MetricsKeys.glyphs").unwrap();

    let ef = font
        .glyphs
        .iter()
        .find(|g| g.glyphname.as_str() == "ef")
        .expect("glyph to be tested wasn't found");
    assert_eq!(ef.metric_left.as_deref(), Some("o"));
    assert_eq!(ef.metric_right.as_deref(), Some("o"));
    assert_eq!(ef.metric_width, None);

    let alphatonos = font
        .glyphs
        .iter()
        .find(|g| g.glyphname.as_str() == "Alphatonos")
        .expect("glyph to be tested wasn't found");
    assert_eq!(alphatonos.metric_left, None);
    assert_eq!(alphatonos.metric_right.as_deref(), Some("A"));
    assert_eq!(alphatonos.metric_width.as_deref(), Some("A"));

    let alphatonos_bold = &alphatonos.layers[1];
    assert_eq!(alphatonos_bold.metric_left.as_deref(), Some("=A"));
    assert_eq!(alphatonos_bold.metric_right, None);
    assert_eq!(alphatonos_bold.metric_width.as_deref(), Some("=123"));
}

#[test]
fn open_contour_smooth_point() {
    // Some glyphs files have open contours with smooth line points. This was
    // erroneously asserted never to occur before.
    let path_source = r#"
        {
            closed = 0;
            nodes = (
                (303,128,ls)
            );
        }
    "#;

    let plist = Plist::parse(path_source).unwrap();
    let path: glyphs_plist::Path = plist.try_into().unwrap();
    let contour = norad::Contour::try_from(&path).unwrap();

    assert!(!contour.is_closed());
    assert_eq!(contour.points, vec![norad::ContourPoint::new(
        303.0,
        128.0,
        norad::PointType::Move,
        true,
        None,
        None,
    )]);
}

#[test]
fn roundtrip_empty_path() {
    // Round-tripping paths without any nodes should not cause panics; they are
    // representable in both file formats and can be loaded or created in
    // Glyphs, even if not directly through the UI.
    let path_source = r#"
        {
            nodes = ();
        }
    "#;

    let plist = Plist::parse(path_source).unwrap();

    let path = glyphs_plist::Path::try_from(plist).unwrap();
    let contour = norad::Contour::try_from(&path).unwrap();
    let path_again = glyphs_plist::Path::from(&contour);

    assert!(path.nodes.is_empty());
    assert_eq!(path, path_again);
}
