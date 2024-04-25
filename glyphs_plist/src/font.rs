//! The general strategy is just to use a plist for storage. Also, lots of
//! unwrapping.
//!
//! There are lots of other ways this could go, including something serde-like
//! where it gets serialized to more Rust-native structures, proc macros, etc.

use std::collections::{BTreeMap, HashMap};
use std::fs;

use kurbo::Point;

use crate::from_plist::FromPlist;
use crate::plist::Plist;
use crate::to_plist::ToPlist;

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Font {
    #[plist(rename = ".appVersion")]
    pub app_version: String,
    #[plist(rename = ".formatVersion")]
    pub format_version: Option<i64>,
    pub date: String,
    pub family_name: String,
    pub version_major: i64,
    pub version_minor: i64,
    pub units_per_em: u16, // Glyphs UI only allows for 16-16384 inclusive
    pub glyphs: Vec<Glyph>,
    pub font_master: Vec<FontMaster>,
    pub metrics: Vec<Metric>,
    pub numbers: Option<Vec<FontNumbers>>,
    pub stems: Option<Vec<FontStems>>,
    pub settings: Option<Settings>,
    pub instances: Option<Vec<Instance>>,
    #[plist(rename = "kerningLTR")]
    pub kerning_ltr: Option<HashMap<String, norad::Kerning>>,
    #[plist(rename = "kerningRTL")]
    pub kerning_rtl: Option<HashMap<String, norad::Kerning>>,
    pub kerning_vertical: Option<HashMap<String, norad::Kerning>>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Metric {
    pub filter: Option<String>,
    pub name: Option<String>,
    pub r#type: Option<MetricType>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MetricType {
    Ascender,
    Baseline,
    BodyHeight,
    CapHeight,
    Descender,
    ItalicAngle,
    MidHeight,
    SlantHeight,
    TopHeight,
    XHeight,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct FontNumbers {
    pub name: String,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct FontStems {
    pub name: String,
    pub filter: Option<String>,
    #[plist(default)]
    pub horizontal: bool,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Settings {
    #[plist(default)]
    pub disables_automatic_alignment: bool,
    #[plist(default)]
    pub disables_nice_names: bool,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Glyph {
    // The Unicode values(s) for the glyph.
    pub unicode: Option<norad::Codepoints>,
    pub layers: Vec<Layer>,
    /// The name of the glyph.
    pub glyphname: norad::Name,
    pub production: Option<String>,
    pub script: Option<String>,
    pub direction: Option<Direction>,
    pub case: Option<Case>,
    pub category: Option<String>,
    pub sub_category: Option<String>,
    #[plist(default)]
    pub tags: Vec<String>,
    // "public.kern1." kerning group, because the right side matters.
    pub kern_right: Option<norad::Name>,
    // "public.kern2." kerning group, because the left side matters.
    pub kern_left: Option<norad::Name>,
    pub kern_top: Option<norad::Name>,
    pub kern_bottom: Option<norad::Name>,
    pub metric_top: Option<String>,
    pub metric_bottom: Option<String>,
    pub metric_left: Option<String>,
    pub metric_right: Option<String>,
    pub metric_width: Option<String>,
    #[plist(default)]
    pub user_data: HashMap<String, Plist>,
    #[plist(default = true)]
    pub export: bool,
    pub color: Option<Color>,
    pub note: Option<String>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Bidi,
    Ltr,
    Rtl,
    Vtl,
    Vtr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Case {
    None,
    Upper,
    Lower,
    SmallCaps,
    Other,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Layer {
    pub attr: Option<LayerAttr>,
    pub name: Option<String>,
    pub background: Option<BackgroundLayer>,
    pub associated_master_id: Option<String>,
    pub layer_id: String,
    pub width: f64,
    pub vert_width: Option<f64>,
    pub vert_origin: Option<f64>,
    #[plist(default)]
    pub shapes: Vec<Shape>,
    pub anchors: Option<Vec<Anchor>>,
    pub guides: Option<Vec<GuideLine>>,
    pub metric_top: Option<String>,
    pub metric_bottom: Option<String>,
    pub metric_left: Option<String>,
    pub metric_right: Option<String>,
    pub metric_width: Option<String>,
    pub metric_vert_width: Option<String>,
    #[plist(default)]
    pub locked: bool,
    #[plist(default)]
    pub user_data: HashMap<String, Plist>,
    pub color: Option<Color>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    Index(i64),
    GreyAlpha(u8, u8),
    Rgba(u8, u8, u8, u8),
    Cmyka(u8, u8, u8, u8, u8),
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct LayerAttr {
    pub axis_rules: Option<Vec<AxisRules>>,
    pub coordinates: Option<Vec<f64>>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct AxisRules {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct BackgroundLayer {
    pub anchors: Option<Vec<Anchor>>,
    #[plist(default)]
    pub shapes: Vec<Shape>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Path(Box<Path>),
    Component(Component),
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Path {
    pub attr: Option<PathAttrs>,
    pub closed: bool,
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct PathAttrs {
    pub line_cap_start: Option<f64>,
    pub line_cap_end: Option<f64>,
    pub stroke_pos: Option<i64>,
    pub stroke_height: Option<f64>,
    pub stroke_width: Option<f64>,
    pub stroke_color: Option<Vec<i64>>,
    pub mask: Option<i64>,
    pub fill: Option<i64>,
    pub fill_color: Option<Vec<i64>>,
    pub shadow: Option<PathShadow>,
    pub gradient: Option<PathGradient>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct PathShadow {
    pub blur: String,
    pub color: Vec<i64>,
    pub offset_x: String,
    pub offset_y: String,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct PathGradient {
    pub colors: Vec<Vec<Color>>, // TODO: Destructure this once relevant.
    pub start: Point,
    pub end: Point,
    pub r#type: String, // TODO: Make enum once relevant.
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
    #[plist(rename = "ref")]
    pub reference: String,
    #[plist(rename = "angle")]
    pub rotation: Option<f64>,
    pub pos: Option<Point>,
    pub scale: Option<Scale>,
    pub slant: Option<Scale>,
    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scale {
    pub horizontal: f64,
    pub vertical: f64,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Anchor {
    pub name: String,
    pub orientation: Option<AnchorOrientation>,
    #[plist(default)]
    pub pos: Point,
    #[plist(default)]
    pub user_data: HashMap<String, Plist>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnchorOrientation {
    Center,
    Right,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct GuideLine {
    pub name: Option<String>,
    #[plist(default)]
    pub angle: f64,
    #[plist(default)]
    pub pos: Point,
    #[plist(default)]
    pub locked: bool,
    #[plist(default)]
    pub lock_angle: f64,
    #[plist(default)]
    pub show_measurement: bool,
    pub orientation: Option<AnchorOrientation>,
    pub filter: Option<String>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct FontMaster {
    pub id: String,
    pub name: String,
    pub metric_values: Vec<MasterMetric>,
    pub number_values: Option<Vec<f64>>,
    pub stem_values: Option<Vec<f64>>,
    pub axes_values: Option<Vec<f64>>,
    #[plist(default = true)]
    pub visible: bool,
    #[plist(default)]
    pub user_data: HashMap<String, Plist>,
    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct MasterMetric {
    pub pos: Option<f64>,
    pub over: Option<f64>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Instance {
    pub name: String,
    pub axes_values: Option<Vec<f64>>,
    #[plist(default = true)]
    pub exports: bool,
    #[plist(default)]
    pub is_bold: bool,
    #[plist(default)]
    pub is_italic: bool,
    pub link_style: Option<String>,
    pub r#type: Option<InstanceType>,
    #[plist(default)]
    pub user_data: HashMap<String, Plist>,
    #[plist(default = true)]
    pub visible: bool,
    #[plist(default = 400)]
    pub weight_class: i64,
    #[plist(default = 5)]
    pub width_class: i64,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstanceType {
    Variable,
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

impl FontMaster {
    /// Iterate over metric "keys" (global) and "values" (per-master).
    ///
    /// If one master does not have a last value that some other master has, the
    /// iterator returns early.
    pub fn iter_metrics<'a>(
        &'a self,
        font: &'a Font,
    ) -> impl Iterator<Item = (&Metric, &MasterMetric)> {
        font.metrics.iter().zip(self.metric_values.iter())
    }
}

impl FromPlist for u16 {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::Integer(wider) => wider
                .try_into()
                .expect("Integer '{:?}' is out-of-bounds of u16"),
            _ => panic!("Cannot parse u16 '{:?}'", plist),
        }
    }
}

impl ToPlist for u16 {
    fn to_plist(self) -> Plist {
        Plist::Integer(self.into())
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

impl FromPlist for Color {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::Integer(int) => Color::Index(int),
            Plist::Array(array) => {
                let numbers: Vec<u8> = array
                    .iter()
                    .map(|v| {
                        v.as_i64()
                            .expect("colors must be numbers")
                            .try_into()
                            .expect("color numbers must be in range 0–255")
                    })
                    .collect();
                match *numbers.as_slice() {
                    [g, a] => Color::GreyAlpha(g, a),
                    [r, g, b, a] => Color::Rgba(r, g, b, a),
                    [c, m, y, k, a] => Color::Cmyka(c, m, y, k, a),
                    _ => panic!(
                        "color array must contain 2 (gray, alpha), 4 (RGBA) or 5 (CMYKA) numbers"
                    ),
                }
            }
            _ => panic!("a color must be either a number or an array of 2–5 numbers"),
        }
    }
}

impl ToPlist for Color {
    fn to_plist(self) -> Plist {
        match self {
            Color::Index(int) => int.into(),
            Color::GreyAlpha(g, a) => Plist::Array(vec![g.into(), a.into()]),
            Color::Rgba(r, g, b, a) => Plist::Array(vec![r.into(), g.into(), b.into(), a.into()]),
            Color::Cmyka(c, m, y, k, a) => {
                Plist::Array(vec![c.into(), m.into(), y.into(), k.into(), a.into()])
            }
        }
    }
}

impl FromPlist for Direction {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::String(s) => match s.as_str() {
                "BIDI" => Direction::Bidi,
                "LTR" => Direction::Ltr,
                "RTL" => Direction::Rtl,
                "VTL" => Direction::Vtl,
                "VTR" => Direction::Vtr,
                _ => panic!("direction must be a string of 'BIDI', 'LTR', 'RTL', 'VTL' or 'VTR'"),
            },
            _ => {
                panic!("direction must be a string of 'BIDI', 'LTR', 'RTL', 'VTL' or 'VTR'")
            }
        }
    }
}

impl ToPlist for Direction {
    fn to_plist(self) -> Plist {
        match self {
            Direction::Bidi => "BIDI".to_string().into(),
            Direction::Ltr => "LTR".to_string().into(),
            Direction::Rtl => "RTL".to_string().into(),
            Direction::Vtl => "VTL".to_string().into(),
            Direction::Vtr => "VTR".to_string().into(),
        }
    }
}

impl FromPlist for Case {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::String(s) => match s.as_str() {
                "noCase" => Case::None,
                "upper" => Case::Upper,
                "lower" => Case::Lower,
                "smallCaps" => Case::SmallCaps,
                "other" => Case::Other,
                _ => panic!(
                    "case must be a string of 'noCase', 'upper', 'lower', 'smallCaps', 'other'"
                ),
            },
            _ => {
                panic!("case must be a string of 'noCase', 'upper', 'lower', 'smallCaps', 'other'")
            }
        }
    }
}

impl ToPlist for Case {
    fn to_plist(self) -> Plist {
        match self {
            Case::None => "noCase".to_string().into(),
            Case::Upper => "upper".to_string().into(),
            Case::Lower => "lower".to_string().into(),
            Case::SmallCaps => "smallCaps".to_string().into(),
            Case::Other => "other".to_string().into(),
        }
    }
}

impl FromPlist for MetricType {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::String(s) => match s.as_str() {
                "ascender" => MetricType::Ascender,
                "baseline" => MetricType::Baseline,
                "bodyHeight" => MetricType::BodyHeight,
                "cap height" => MetricType::CapHeight,
                "descender" => MetricType::Descender,
                "italic angle" => MetricType::ItalicAngle,
                "midHeight" => MetricType::MidHeight,
                "slant height" => MetricType::SlantHeight,
                "topHeight" => MetricType::TopHeight,
                "x-height" => MetricType::XHeight,
                _ => panic!("metric type must be a string of 'ascender', 'cap height', 'slant height', 'x-height', 'midHeight', 'topHeight', 'bodyHeight', 'descender', 'baseline', 'italic angle'"),
            },
            _ => {
                panic!("metric type must be a string of 'ascender', 'cap height', 'slant height', 'x-height', 'midHeight', 'topHeight', 'bodyHeight', 'descender', 'baseline', 'italic angle'")
            }
        }
    }
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Ascender => write!(f, "ascender"),
            MetricType::Baseline => write!(f, "baseline"),
            MetricType::BodyHeight => write!(f, "bodyHeight"),
            MetricType::CapHeight => write!(f, "cap height"),
            MetricType::Descender => write!(f, "descender"),
            MetricType::ItalicAngle => write!(f, "italic angle"),
            MetricType::MidHeight => write!(f, "midHeight"),
            MetricType::SlantHeight => write!(f, "slant height"),
            MetricType::TopHeight => write!(f, "topHeight"),
            MetricType::XHeight => write!(f, "x-height"),
        }
    }
}

impl ToPlist for MetricType {
    fn to_plist(self) -> Plist {
        self.to_string().into()
    }
}

impl FromPlist for InstanceType {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::String(s) => match s.as_str() {
                "variable" => InstanceType::Variable,
                _ => panic!("instance type type must be a string of 'variable'"),
            },
            _ => {
                panic!("instance type type must be a string of 'variable'")
            }
        }
    }
}

impl ToPlist for InstanceType {
    fn to_plist(self) -> Plist {
        match self {
            InstanceType::Variable => "variable".to_string().into(),
        }
    }
}

impl FromPlist for Shape {
    fn from_plist(plist: Plist) -> Self {
        match plist {
            Plist::Dictionary(dict) => {
                if dict.contains_key("ref") {
                    Shape::Component(FromPlist::from_plist(Plist::Dictionary(dict)))
                } else {
                    Shape::Path(Box::new(FromPlist::from_plist(Plist::Dictionary(dict))))
                }
            }
            _ => panic!("Cannot parse shape '{:?}'", plist),
        }
    }
}

impl ToPlist for Shape {
    fn to_plist(self) -> Plist {
        match self {
            Shape::Path(path) => ToPlist::to_plist(*path),
            Shape::Component(component) => ToPlist::to_plist(component),
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
        const ERR_MSG: &str = "Unicode codepoint must be integer in range U+0000–U+10FFFF";
        match plist {
            Plist::Integer(n) => {
                let cp: u32 = n.try_into().expect(ERR_MSG);
                let cp = char::try_from(cp).expect(ERR_MSG);
                norad::Codepoints::new([cp])
            }
            Plist::Array(array) => {
                norad::Codepoints::new(array.iter().map(|codepoint| match codepoint {
                    Plist::Integer(n) => {
                        let cp: u32 = (*n).try_into().expect(ERR_MSG);
                        char::try_from(cp).expect(ERR_MSG)
                    }
                    _ => panic!("codepoint must be integer, but got {:?}", codepoint),
                }))
            }
            _ => panic!(
                "codepoint must be integer or array of integers, but got {:?}",
                plist
            ),
        }
    }
}

impl ToPlist for norad::Codepoints {
    fn to_plist(self) -> Plist {
        assert!(!self.is_empty());
        if self.len() == 1 {
            Plist::Integer(self.iter().next().unwrap() as i64)
        } else {
            Plist::Array(self.iter().map(|cp| Plist::Integer(cp as i64)).collect())
        }
    }
}

impl FromPlist for Node {
    fn from_plist(plist: Plist) -> Self {
        let mut tuple = plist
            .as_array()
            .expect("a node must be described by a tuple")
            .iter();
        let x = tuple
            .next()
            .expect("a node must have an x coordinate")
            .as_f64()
            .expect("a node x coordinate must be a floating point number");
        let y = tuple
            .next()
            .expect("a node must have a y coordinate")
            .as_f64()
            .expect("a node y coordinate must be a floating point number");
        let node_type = tuple
            .next()
            .expect("a node must have type")
            .as_str()
            .expect("a node type must be a string")
            .parse()
            .expect("a node type must be any of 'l', 'ls', 'c', 'cs', 'q', 'qs' or 'o'");

        let pt = Point::new(x, y);
        Node { pt, node_type }
    }
}

impl std::str::FromStr for NodeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "l" => Ok(NodeType::Line),
            "ls" => Ok(NodeType::LineSmooth),
            "c" => Ok(NodeType::Curve),
            "cs" => Ok(NodeType::CurveSmooth),
            "q" => Ok(NodeType::QCurve),
            "qs" => Ok(NodeType::QCurveSmooth),
            "o" => Ok(NodeType::OffCurve),
            _ => Err(format!("unknown node type {}", s)),
        }
    }
}

impl NodeType {
    fn glyphs_str(&self) -> &'static str {
        match self {
            NodeType::Line => "l",
            NodeType::LineSmooth => "ls",
            NodeType::Curve => "c",
            NodeType::CurveSmooth => "cs",
            NodeType::QCurve => "q",
            NodeType::QCurveSmooth => "qs",
            NodeType::OffCurve => "o",
        }
    }
}

impl ToPlist for Node {
    fn to_plist(self) -> Plist {
        Plist::Array(vec![
            self.pt.x.into(),
            self.pt.y.into(),
            self.node_type.glyphs_str().to_string().into(),
        ])
    }
}

impl FromPlist for Point {
    fn from_plist(plist: Plist) -> Self {
        let mut raw = plist
            .as_array()
            .expect("point must be described by a tuple")
            .iter()
            .map(|v| v.as_f64().expect("coordinate must be a number"));
        let x = raw.next().expect("point must have an x coordinate");
        let y = raw.next().expect("point must have a y coordinate");
        assert!(
            raw.next().is_none(),
            "point must have exactly two coordinates"
        );
        Point::new(x, y)
    }
}

impl ToPlist for Point {
    fn to_plist(self) -> Plist {
        Plist::Array(vec![self.x.into(), self.y.into()])
    }
}

impl FromPlist for Scale {
    fn from_plist(plist: Plist) -> Self {
        let mut raw = plist
            .as_array()
            .expect("scale must be described by a tuple")
            .iter()
            .map(|v| v.as_f64().expect("scale value must be a number"));
        let horizontal = raw.next().expect("scale must have a horizontal value");
        let vertical = raw.next().expect("scale must have a vertical value");
        assert!(raw.next().is_none(), "scale must have exactly two values");
        Self {
            horizontal,
            vertical,
        }
    }
}

impl ToPlist for Scale {
    fn to_plist(self) -> Plist {
        Plist::Array(vec![self.horizontal.into(), self.vertical.into()])
    }
}

impl Path {
    pub fn new(closed: bool) -> Path {
        Path {
            attr: None,
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
        let font = Font::load("testdata/GlyphsFileFormatv3.glyphs").unwrap();

        assert_eq!(font.app_version, "3259");
        assert_eq!(font.format_version, Some(3));

        assert!(!font.other_stuff.contains_key(".appVersion"));
        assert!(!font.other_stuff.contains_key(".formatVersion"));
    }

    // TODO: Need to be able to skip serializing default values for this.
    // #[test]
    // fn roundtrip_plist() {
    //     let contents = std::fs::read_to_string("testdata/NewFontG3.glyphs").unwrap();
    //     let plist = Plist::parse(&contents).unwrap();
    //     let plist_original = plist.clone();
    //     let font = Font::from_plist(plist);
    //     let plist_roundtrip = ToPlist::to_plist(font);
    //
    //     assert_eq!(plist_original, plist_roundtrip);
    // }

    #[test]
    fn only_expected_other_stuff() {
        // TODO: Run on all test fixtures.
        let font = Font::load("testdata/GlyphsFileFormatv3.glyphs").unwrap();

        let other_keys: HashSet<String> = font.other_stuff.keys().cloned().collect();

        let disallowed: HashSet<String> = other_keys
            .difference(&HashSet::from([
                // Explicitly unhandled:
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
    }

    #[test]
    #[should_panic(expected = r#"unrecognised fields in FooBar: ["bar"]"#)]
    fn panics_on_unexpected_fields() {
        #[derive(FromPlist)]
        struct FooBar {
            _foo: String,
        }

        let with_unexpected = Plist::Dictionary(HashMap::from([
            ("foo".to_owned(), Plist::String("abc".to_owned())),
            ("bar".to_owned(), Plist::String("def".to_owned())),
        ]));

        FooBar::from_plist(with_unexpected);
    }
}
