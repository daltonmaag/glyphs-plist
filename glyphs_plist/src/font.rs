//! The general strategy is just to use a plist for storage. Also, lots of
//! unwrapping.
//!
//! There are lots of other ways this could go, including something serde-like
//! where it gets serialized to more Rust-native structures, proc macros, etc.

use std::collections::{BTreeMap, HashMap};
use std::fs;

use kurbo::{Affine, Point};

use crate::from_plist::FromPlist;
use crate::plist::Plist;
use crate::to_plist::ToPlist;

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Font {
    pub family_name: String,
    pub version_major: i64,
    pub version_minor: i64,
    pub units_per_em: f64,
    pub glyphs: Vec<Glyph>,
    pub font_master: Vec<FontMaster>,
    pub instances: Option<Vec<Instance>>,
    #[rename("kerningLTR")]
    pub kerning_ltr: Option<HashMap<String, norad::Kerning>>,
    #[rename("kerningRTL")]
    pub kerning_rtl: Option<HashMap<String, norad::Kerning>>,
    pub disables_automatic_alignment: Option<bool>,
    pub disables_nice_names: Option<bool>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Glyph {
    // The Unicode values(s) for the glyph.
    pub unicode: Option<norad::Codepoints>,
    pub layers: Vec<Layer>,
    /// The name of the glyph.
    pub glyphname: norad::Name,
    // "public.kern1." kerning group, because the right side matters.
    pub kern_right: Option<norad::Name>,
    // "public.kern2." kerning group, because the left side matters.
    pub kern_left: Option<norad::Name>,
    pub metric_left: Option<String>,
    pub metric_right: Option<String>,
    pub metric_width: Option<String>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Layer {
    pub name: Option<String>,
    pub background: Option<BackgroundLayer>,
    pub associated_master_id: Option<String>,
    pub layer_id: String,
    pub width: f64,
    pub paths: Option<Vec<Path>>,
    pub components: Option<Vec<Component>>,
    pub anchors: Option<Vec<Anchor>>,
    pub guide_lines: Option<Vec<GuideLine>>,
    pub metric_left: Option<String>,
    pub metric_right: Option<String>,
    pub metric_width: Option<String>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct BackgroundLayer {
    pub paths: Option<Vec<Path>>,
    pub components: Option<Vec<Component>>,
    pub anchors: Option<Vec<Anchor>>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Path {
    pub closed: bool,
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub pt: Point,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    Line,
    LineSmooth,
    OffCurve,
    Curve,
    CurveSmooth,
    QCurve,
    QCurveSmooth,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Component {
    pub name: String,
    pub transform: Option<Affine>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Anchor {
    pub name: String,
    pub orientation: Option<AnchorOrientation>,
    pub pos: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnchorOrientation {
    Center,
    Right,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct GuideLine {
    pub angle: Option<f64>,
    pub pos: Point,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct FontMaster {
    pub id: String,
    pub alignment_zones: Option<Vec<AlignmentZone>>,
    pub ascender: Option<i64>,
    pub cap_height: Option<i64>,
    pub descender: Option<i64>,
    pub x_height: Option<i64>,
    pub italic_angle: Option<f64>,
    // Glyphs.app 2.x will truncate floating point coordinates for sources to
    // integers, 3.x will keep them as is. Likely an edge case, and we're moving
    // to 3.x, anyway.
    pub weight_value: Option<f64>,
    pub width_value: Option<f64>,
    pub custom_value: Option<f64>,
    pub custom_value1: Option<f64>,
    pub custom_value2: Option<f64>,
    pub custom_value3: Option<f64>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlignmentZone {
    pub position: f64,
    pub size: f64,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Instance {
    pub name: String,
    pub interpolation_weight: Option<f64>,
    pub interpolation_width: Option<f64>,
    pub interpolation_custom: Option<f64>,
    pub interpolation_custom1: Option<f64>,
    pub interpolation_custom2: Option<f64>,
    pub interpolation_custom3: Option<f64>,
    pub is_bold: Option<bool>,
    pub is_italic: Option<bool>,
    pub link_style: Option<String>,
    #[rest]
    pub other_stuff: HashMap<String, Plist>,
}

impl Font {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Font, String> {
        let contents = std::fs::read_to_string(path).map_err(|e| format!("{:?}", e))?;
        let plist = Plist::parse(&contents).map_err(|e| format!("{:?}", e))?;

        // The formatVersion key is only present in Glyphs 3+ files.
        if plist.get(".formatVersion").is_none() {
            return Err("Glyphs 2 files are not currently supported. \n\n\
                        Go to Font Info, click the 'Other' tab and set 'File format version' to 'Version 3'."
                .to_string());
        }

        Ok(FromPlist::from_plist(plist))
    }

    pub fn save(self, path: &std::path::Path) -> Result<(), String> {
        let plist = self.to_plist();
        fs::write(path, plist.to_string()).map_err(|e| format!("{:?}", e))
    }

    pub fn get_glyph(&self, glyphname: &str) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.glyphname == glyphname)
    }

    pub fn get_glyph_mut(&mut self, glyphname: &str) -> Option<&mut Glyph> {
        self.glyphs.iter_mut().find(|g| g.glyphname == glyphname)
    }
}

impl Glyph {
    pub fn get_layer(&self, layer_id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.layer_id == layer_id)
    }
}

impl FromPlist for norad::Name {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::String(s) => Self::new(s.as_str())
                .unwrap_or_else(|e| panic!("Cannot parse glyphname '{}': {:?}", s, e)),
            // Due to Glyphs.app quirks removing quotes around the name "infinity",
            // it is parsed as a float instead.
            Plist::Float(f) if f.is_infinite() => Self::new("infinity").unwrap(),
            Plist::Float(f) if f.is_nan() => Self::new("nan").unwrap(),
            _ => panic!("Cannot parse glyphname '{:?}'", plist),
        }
    }
}

impl FromPlist for AnchorOrientation {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::String(s) => match s.as_str() {
                "center" => AnchorOrientation::Center,
                "right" => AnchorOrientation::Right,
                _ => panic!("Unknown anchor orientation '{:?}'", s),
            },
            _ => panic!("Cannot parse anchor orientation '{:?}'", plist),
        }
    }
}

impl ToPlist for AnchorOrientation {
    fn to_plist(self) -> Plist {
        match self {
            AnchorOrientation::Center => Plist::String("center".into()),
            AnchorOrientation::Right => Plist::String("right".into()),
        }
    }
}

impl ToPlist for norad::Name {
    fn to_plist(self) -> Plist {
        self.to_string().into()
    }
}

impl FromPlist for norad::Codepoints {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::Integer(n) => {
                let cp: u32 = n.try_into().expect("Cannot parse codepoint");
                let cp = char::try_from(cp).expect("Cannot parse codepoint");
                norad::Codepoints::new([cp])
            }
            Plist::Array(array) => {
                norad::Codepoints::new(array.iter().map(|codepoint| match codepoint {
                    Plist::Integer(n) => {
                        let cp: u32 = (*n).try_into().expect("Cannot parse codepoint");
                        char::try_from(cp).expect("Cannot parse codepoint")
                    }
                    _ => panic!("Cannot parse codepoint: {:?}", codepoint),
                }))
            }
            _ => panic!("Cannot parse codepoints: {:?}", plist),
        }
    }
}

impl ToPlist for norad::Codepoints {
    fn to_plist(self) -> Plist {
        assert!(!self.is_empty());
        self.iter()
            .map(|c| format!("{:04X}", c as usize))
            .collect::<Vec<_>>()
            .join(",")
            .into()
    }
}

impl FromPlist for Node {
    fn from_plist(plist: Plist) -> Self {
        let mut spl = plist.as_str().unwrap().splitn(3, ' ');
        let x = spl.next().unwrap().parse().unwrap();
        let y = spl.next().unwrap().parse().unwrap();
        let pt = Point::new(x, y);
        let node_type = spl.next().unwrap().parse().unwrap();
        Node { pt, node_type }
    }
}

impl std::str::FromStr for NodeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LINE" => Ok(NodeType::Line),
            "LINE SMOOTH" => Ok(NodeType::LineSmooth),
            "OFFCURVE" => Ok(NodeType::OffCurve),
            "CURVE" => Ok(NodeType::Curve),
            "CURVE SMOOTH" => Ok(NodeType::CurveSmooth),
            "QCURVE" => Ok(NodeType::QCurve),
            "QCURVE SMOOTH" => Ok(NodeType::QCurveSmooth),
            _ => Err(format!("unknown node type {}", s)),
        }
    }
}

impl NodeType {
    fn glyphs_str(&self) -> &'static str {
        match self {
            NodeType::Line => "LINE",
            NodeType::LineSmooth => "LINE SMOOTH",
            NodeType::OffCurve => "OFFCURVE",
            NodeType::Curve => "CURVE",
            NodeType::CurveSmooth => "CURVE SMOOTH",
            NodeType::QCurve => "QCURVE",
            NodeType::QCurveSmooth => "QCURVE SMOOTH",
        }
    }
}

impl ToPlist for Node {
    fn to_plist(self) -> Plist {
        format!(
            "{} {} {}",
            self.pt.x,
            self.pt.y,
            self.node_type.glyphs_str()
        )
        .into()
    }
}

impl FromPlist for Affine {
    fn from_plist(plist: Plist) -> Self {
        let raw = plist.as_str().unwrap();
        let raw = &raw[1..raw.len() - 1];
        let coords: Vec<f64> = raw.split(", ").map(|c| c.parse().unwrap()).collect();
        Affine::new([
            coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
        ])
    }
}

impl ToPlist for Affine {
    fn to_plist(self) -> Plist {
        let c = self.as_coeffs();
        format!(
            "{{{}, {}, {}, {}, {}, {}}}",
            c[0], c[1], c[2], c[3], c[4], c[5]
        )
        .into()
    }
}

impl FromPlist for Point {
    fn from_plist(plist: Plist) -> Self {
        let raw = plist.as_array().expect("Cannot parse point tuple");
        let mut raw = raw.iter().map(|v| v.as_f64().expect("Cannot parse point"));
        let x = raw.next().expect("No x coordinate");
        let y = raw.next().expect("No y coordinate");
        assert!(raw.next().is_none(), "Too many coordinates");
        Point::new(x, y)
    }
}

impl ToPlist for Point {
    fn to_plist(self) -> Plist {
        format!("{{{}, {}}}", self.x, self.y).into()
    }
}

impl Path {
    pub fn new(closed: bool) -> Path {
        Path {
            nodes: Vec::new(),
            closed,
        }
    }

    pub fn add(&mut self, pt: impl Into<Point>, node_type: NodeType) {
        let pt = pt.into();
        self.nodes.push(Node { pt, node_type });
    }

    /// Rotate left by one, placing the first point at the end. This is because
    /// it's what glyphs seems to expect.
    pub fn rotate_left(&mut self, delta: usize) {
        self.nodes.rotate_left(delta);
    }

    pub fn reverse(&mut self) {
        self.nodes.reverse();
    }
}

impl FontMaster {
    pub fn name(&self) -> &str {
        self.other_stuff
            .get("customParameters")
            .map(|cps| {
                cps.as_array()
                    .unwrap()
                    .iter()
                    .map(|cp| cp.as_dict().unwrap())
            })
            .and_then(|mut cps| {
                cps.find(|cp| cp.get("name").unwrap().as_str().unwrap() == "Master Name")
            })
            .and_then(|cp| cp.get("value").unwrap().as_str())
            .expect("Cannot determine name for master")
    }
}

impl ToPlist for HashMap<String, norad::Kerning> {
    fn to_plist(self) -> Plist {
        let mut kerning = HashMap::new();

        for (master_id, master_kerning) in self {
            let mut first_dict = HashMap::new();
            for (first, second_map) in master_kerning {
                let mut second_dict = HashMap::new();
                for (second, value) in second_map {
                    second_dict.insert(second.to_string(), value.into());
                }
                first_dict.insert(first.to_string(), second_dict.into());
            }
            kerning.insert(master_id.clone(), first_dict.into());
        }

        Plist::Dictionary(kerning)
    }
}

impl FromPlist for HashMap<String, norad::Kerning> {
    fn from_plist(plist: Plist) -> Self {
        let mut kerning = HashMap::new();

        for (master_id, master_kerning) in plist.as_dict().unwrap() {
            let mut new_master_kerning = norad::Kerning::new();
            for (left, second_dict) in master_kerning.as_dict().unwrap() {
                let mut new_second_dict = BTreeMap::new();
                for (right, value) in second_dict.as_dict().unwrap() {
                    let value = value.as_f64().unwrap();
                    new_second_dict.insert(norad::Name::new(right).unwrap(), value);
                }
                new_master_kerning.insert(norad::Name::new(left).unwrap(), new_second_dict);
            }
            kerning.insert(master_id.clone(), new_master_kerning);
        }

        kerning
    }
}

impl FromPlist for AlignmentZone {
    fn from_plist(plist: Plist) -> Self {
        if let Plist::String(string) = plist {
            let string = string
                .strip_prefix('{')
                .expect("Alignment zone must start with a '{'")
                .strip_suffix('}')
                .expect("Alignment zone must end with a '}'");
            let mut iter = string.split(',').map(|s| s.trim());
            let position = iter
                .next()
                .expect("Need two numbers in alignment zone")
                .parse()
                .expect("Alignment zone position must be a number");
            let size = iter
                .next()
                .expect("Need two numbers in alignment zone")
                .parse()
                .expect("Alignment zone size must be a number");
            assert!(
                iter.next().is_none(),
                "An alignment zone must have at most two numbers"
            );
            AlignmentZone { position, size }
        } else {
            panic!("Alignment zone {:?} must be a string", plist);
        }
    }
}

impl ToPlist for AlignmentZone {
    fn to_plist(self) -> Plist {
        Plist::String(format!("{{{}, {}}}", self.position, self.size))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn parse_empty_font_glyphs2() {
        Font::load("testdata/NewFont.glyphs").unwrap_err();
    }

    #[test]
    fn parse_empty_font_glyphs3() {
        Font::load("testdata/NewFontG3.glyphs").unwrap();
    }

    #[test]
    fn parse_float_names() {
        Font::load("testdata/FloatNames.glyphs").unwrap();
    }

    #[test]
    fn parse_format3_example() {
        Font::load("testdata/GlyphsFileFormatv3.glyphs").unwrap();
    }

    #[test]
    fn only_expected_other_stuff() {
        // TODO: Run on all test fixtures.
        let font = Font::load("testdata/GlyphsFileFormatv3.glyphs").unwrap();

        let other_keys: HashSet<String> = font.other_stuff.keys().cloned().collect();

        let disallowed: HashSet<String> = other_keys
            .difference(&HashSet::from([
                // Explicitly unhandled:
                ".appVersion".to_owned(),
                ".formatVersion".to_owned(),
                "features".to_owned(),
                "featurePrefixes".to_owned(),
                // Potentially should be handled:
                // TODO: Evaluate these.
                "numbers".to_owned(),
                "kerningVertical".to_owned(),
                "customParameters".to_owned(),
                "properties".to_owned(),
                "DisplayStrings".to_owned(),
                "classes".to_owned(),
                "userData".to_owned(),
                "stems".to_owned(),
                "metrics".to_owned(),
                "settings".to_owned(),
                "note".to_owned(),
                "axes".to_owned(),
                "date".to_owned(),
            ]))
            .cloned()
            .collect();

        assert_eq!(disallowed, HashSet::from([]));

        // TODO: Implement for nested structs.
        // TODO: Check that structs without #[rest] fail to parse when there are extra keys.
    }
}
