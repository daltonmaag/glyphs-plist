//! The general strategy is just to use a plist for storage. Also, lots of
//! unwrapping.
//!
//! There are lots of other ways this could go, including something serde-like
//! where it gets serialized to more Rust-native structures, proc macros, etc.

use std::collections::HashMap;
use std::convert::Infallible;
use std::{fs, io};

use kurbo::Point;
use thiserror::Error;

use crate::from_plist::{
    ArrayConversionError, BoolConversionError, DownsizeToU16Error, FromPlist, VariantError,
};
use crate::plist::Plist;
use crate::to_plist::ToPlist;

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Font {
    #[plist(rename = ".appVersion", always_serialise)]
    pub app_version: String,
    #[plist(rename = ".formatVersion", always_serialise)]
    pub format_version: Option<i64>,
    #[plist(always_serialise)]
    pub date: String,
    #[plist(always_serialise)]
    pub family_name: String,
    #[plist(always_serialise)]
    pub version_major: i64,
    #[plist(always_serialise)]
    pub version_minor: i64,
    #[plist(always_serialise)]
    pub units_per_em: u16, // Glyphs UI only allows for 16-16384 inclusive
    #[plist(always_serialise)]
    pub glyphs: Vec<Glyph>,
    #[plist(always_serialise)]
    pub font_master: Vec<FontMaster>,
    #[plist(always_serialise)]
    pub metrics: Vec<Metric>,
    pub axes: Option<Vec<Axis>>,
    pub numbers: Option<Vec<FontNumbers>>,
    pub stems: Option<Vec<FontStems>>,
    pub settings: Option<Settings>,
    pub instances: Option<Vec<Instance>>,
    #[plist(rename = "kerningLTR")]
    pub kerning_ltr: Option<HashMap<String, norad::Kerning>>,
    #[plist(rename = "kerningRTL")]
    pub kerning_rtl: Option<HashMap<String, norad::Kerning>>,
    pub kerning_vertical: Option<HashMap<String, norad::Kerning>>,
    pub user_data: Option<HashMap<String, Plist>>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Axis {
    #[plist(always_serialise)]
    pub name: String,
    #[plist(always_serialise)]
    pub tag: String,
    #[plist(default)]
    pub hidden: bool,
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

#[derive(Clone, Debug, Default, FromPlist, ToPlist, PartialEq)]
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
    #[plist(always_serialise)]
    pub glyphname: norad::Name,
    // The Unicode values(s) for the glyph.
    pub unicode: Option<norad::Codepoints>,
    #[plist(always_serialise)]
    pub layers: Vec<Layer>,
    /// The name of the glyph.
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
    #[plist(default)]
    pub locked: bool,

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
    #[plist(always_serialise)]
    pub layer_id: String,
    #[plist(always_serialise)]
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
    #[plist(always_serialise, default = true)]
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
    pub attr: Option<NodeAttrs>,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct NodeAttrs {
    pub name: Option<String>,

    #[plist(rest)]
    pub other_stuff: HashMap<String, Plist>,
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
    #[plist(rename = "ref", always_serialise)]
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
    #[plist(always_serialise)]
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
    #[plist(always_serialise)]
    pub id: String,
    #[plist(always_serialise)]
    pub name: String,
    #[plist(always_serialise)]
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
    #[plist(default)]
    pub pos: f64,
    #[plist(default)]
    pub over: f64,
}

#[derive(Clone, Debug, FromPlist, ToPlist, PartialEq)]
pub struct Instance {
    #[plist(always_serialise)]
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

impl Default for Font {
    fn default() -> Self {
        Self {
            app_version: "3259".to_string(),
            date: "2024-04-25 08:35:58 +0000".to_string(),
            format_version: Some(3),
            family_name: "New Font".to_string(),
            version_major: 1,
            version_minor: Default::default(),
            units_per_em: 1000,
            glyphs: vec![Glyph {
                layers: vec![Layer {
                    width: 200.0,
                    ..Layer::new("m01", None)
                }],
                ..Glyph::new(
                    norad::Name::new("space").unwrap(),
                    Some(norad::Codepoints::new(vec![' '])),
                )
            }],
            font_master: vec![FontMaster {
                metric_values: vec![
                    MasterMetric {
                        pos: 800.0,
                        over: 16.0,
                    },
                    MasterMetric {
                        pos: 0.0,
                        over: -16.0,
                    },
                    MasterMetric {
                        pos: -200.0,
                        over: -16.0,
                    },
                ],
                ..FontMaster::new("m01", "Regular")
            }],
            metrics: vec![
                Metric {
                    filter: None,
                    name: None,
                    r#type: Some(MetricType::Ascender),
                },
                Metric {
                    filter: None,
                    name: None,
                    r#type: Some(MetricType::Baseline),
                },
                Metric {
                    filter: None,
                    name: None,
                    r#type: Some(MetricType::Descender),
                },
            ],
            axes: Default::default(),
            numbers: Default::default(),
            stems: Default::default(),
            settings: Default::default(),
            instances: Default::default(),
            kerning_ltr: Default::default(),
            kerning_rtl: Default::default(),
            kerning_vertical: Default::default(),
            user_data: Default::default(),
            other_stuff: Default::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum FontLoadError {
    #[error("failed to read file: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse file as plist: {0}")]
    ParsePlist(#[from] crate::plist::Error),
    #[error("Glyphs 2 files are not supported")]
    Glyphs2,
    #[error(transparent)]
    ParseGlyphs(#[from] GlyphsFromPlistError),
}

impl Font {
    /// Return a new font like Glyphs.app would do it.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Font, FontLoadError> {
        let contents = fs::read_to_string(path)?;
        let plist = Plist::parse(&contents)?;

        // The formatVersion key is only present in Glyphs 3+ files.
        if plist.get(".formatVersion").is_none() {
            return Err(FontLoadError::Glyphs2);
        }

        Ok(plist.try_into()?)
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
    pub fn new(glyphname: impl Into<norad::Name>, unicodes: Option<norad::Codepoints>) -> Self {
        Self {
            glyphname: glyphname.into(),
            unicode: unicodes,
            case: None,
            category: None,
            color: None,
            direction: None,
            export: true,
            kern_bottom: None,
            kern_left: None,
            kern_right: None,
            kern_top: None,
            layers: vec![],
            locked: false,
            metric_bottom: None,
            metric_left: None,
            metric_right: None,
            metric_top: None,
            metric_width: None,
            note: None,
            other_stuff: Default::default(),
            production: None,
            script: None,
            sub_category: None,
            tags: Default::default(),
            user_data: Default::default(),
        }
    }

    pub fn get_layer(&self, layer_id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.layer_id == layer_id)
    }
}

impl Layer {
    pub fn new(layer_id: impl Into<String>, associated_master_id: Option<String>) -> Self {
        Self {
            attr: Default::default(),
            name: Default::default(),
            background: Default::default(),
            associated_master_id: associated_master_id.map(Into::into),
            layer_id: layer_id.into(),
            width: 600.0,
            vert_width: Default::default(),
            vert_origin: Default::default(),
            shapes: Default::default(),
            anchors: Default::default(),
            guides: Default::default(),
            metric_top: Default::default(),
            metric_bottom: Default::default(),
            metric_left: Default::default(),
            metric_right: Default::default(),
            metric_width: Default::default(),
            metric_vert_width: Default::default(),
            user_data: Default::default(),
            color: Default::default(),
            other_stuff: Default::default(),
        }
    }

    pub fn is_master_layer(&self) -> bool {
        self.associated_master_id.is_none()
    }

    pub fn is_intermediate_layer(&self) -> bool {
        self.attr
            .as_ref()
            .map(|attr| attr.coordinates.is_some())
            .unwrap_or(false)
    }

    pub fn is_alternate_layer(&self) -> bool {
        self.attr
            .as_ref()
            .map(|attr| attr.axis_rules.is_some())
            .unwrap_or(false)
    }

    pub fn is_color_layer(&self) -> bool {
        self.attr
            .as_ref()
            .map(|attr| attr.other_stuff.contains_key("color"))
            .unwrap_or(false)
    }

    pub fn is_color_palette_layer(&self) -> bool {
        self.attr
            .as_ref()
            .map(|attr| attr.other_stuff.contains_key("colorPalette"))
            .unwrap_or(false)
    }

    pub fn is_svg_layer(&self) -> bool {
        self.attr
            .as_ref()
            .map(|attr| attr.other_stuff.contains_key("svg"))
            .unwrap_or(false)
    }

    pub fn is_icolor_layer(&self) -> bool {
        self.attr
            .as_ref()
            .map(|attr| attr.other_stuff.contains_key("sbixSize"))
            .unwrap_or(false)
    }

    pub fn coordinates(&self) -> Option<&[f64]> {
        self.attr.as_ref().and_then(|a| a.coordinates.as_deref())
    }
}

impl FontMaster {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            metric_values: Default::default(),
            number_values: Default::default(),
            stem_values: Default::default(),
            axes_values: Default::default(),
            visible: true,
            user_data: Default::default(),
            other_stuff: Default::default(),
        }
    }

    /// Iterate over metric "keys" (global) and "values" (per-master).
    ///
    /// If one master does not have a last value that some other master has, the
    /// iterator returns early.
    pub fn iter_metrics<'a>(
        &'a self,
        font: &'a Font,
    ) -> impl Iterator<Item = (&'a Metric, &'a MasterMetric)> {
        font.metrics.iter().zip(self.metric_values.iter())
    }
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Instance {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            axes_values: Default::default(),
            exports: true,
            is_bold: Default::default(),
            is_italic: Default::default(),
            link_style: Default::default(),
            other_stuff: Default::default(),
            r#type: Default::default(),
            user_data: Default::default(),
            visible: true,
            weight_class: Default::default(),
            width_class: Default::default(),
        }
    }
}

#[derive(Debug, Error)]
#[error("name must be a string or a float with value infinite/NaN")]
pub struct NameConversionError;

impl TryFrom<Plist> for norad::Name {
    type Error = NameConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::String(s) => Self::new(s.as_str())
                .map_err(|_| panic!("glyph name {s:?} valid in Glyphs but not norad")),
            // Due to Glyphs.app quirks removing quotes around the name "infinity",
            // it is parsed as a float instead.
            Plist::Float(f) if f.is_infinite() => Ok(Self::new("infinity").unwrap()),
            Plist::Float(f) if f.is_nan() => Ok(Self::new("nan").unwrap()),
            _ => Err(NameConversionError),
        }
    }
}

#[derive(Debug, Error)]
pub enum AnchorOrientationConversionError {
    #[error("can't convert non-string plist value to AnchorOrientation")]
    WrongVariant,
    #[error("unknown anchor orientation {0:?}")]
    UnknownOrientation(String),
}

impl TryFrom<Plist> for AnchorOrientation {
    type Error = AnchorOrientationConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::String(s) => match s.as_str() {
                "center" => Ok(AnchorOrientation::Center),
                "right" => Ok(AnchorOrientation::Right),
                _ => Err(AnchorOrientationConversionError::UnknownOrientation(s)),
            },
            _ => Err(AnchorOrientationConversionError::WrongVariant),
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

#[derive(Debug, Error)]
pub enum ColorConversionError {
    #[error("color can only be parsed from an integer or integer array")]
    WrongVariant,
    #[error("color array must contain 2 (gray, alpha), 4 (RGBA) or 5 (CMYKA) numbers")]
    UnsupportedArray,
    #[error("{0} is out-of-bounds for a u8")]
    OutOfBounds(i64),
}

impl TryFrom<Plist> for Color {
    type Error = ColorConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::Integer(int) => Ok(Color::Index(int)),
            Plist::Array(array) => {
                let numbers: Result<Vec<u8>, _> = array
                    .iter()
                    .map(|v| {
                        let n = v.as_i64().ok_or(ColorConversionError::WrongVariant)?;
                        n.try_into()
                            .map_err(|_| ColorConversionError::OutOfBounds(n))
                    })
                    .collect();
                match *numbers?.as_slice() {
                    [g, a] => Ok(Color::GreyAlpha(g, a)),
                    [r, g, b, a] => Ok(Color::Rgba(r, g, b, a)),
                    [c, m, y, k, a] => Ok(Color::Cmyka(c, m, y, k, a)),
                    _ => Err(ColorConversionError::UnsupportedArray),
                }
            }
            _ => Err(ColorConversionError::WrongVariant),
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

#[derive(Debug, Error)]
#[error(r#"direction must be a string containing only "BIDI", "LTR", "RTL", "VTL", or "VTR""#)]
pub struct DirectionConversionError;

impl TryFrom<Plist> for Direction {
    type Error = DirectionConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::String(s) => match s.as_str() {
                "BIDI" => Ok(Direction::Bidi),
                "LTR" => Ok(Direction::Ltr),
                "RTL" => Ok(Direction::Rtl),
                "VTL" => Ok(Direction::Vtl),
                "VTR" => Ok(Direction::Vtr),
                _ => Err(DirectionConversionError),
            },
            _ => Err(DirectionConversionError),
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

#[derive(Debug, Error)]
#[error(
    r#"case must be a string containing only "noCase", "upper", "lower", "smallCaps", or "other""#
)]
pub struct CaseConversionError;

impl TryFrom<Plist> for Case {
    type Error = CaseConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::String(s) => match s.as_str() {
                "noCase" => Ok(Case::None),
                "upper" => Ok(Case::Upper),
                "lower" => Ok(Case::Lower),
                "smallCaps" => Ok(Case::SmallCaps),
                "other" => Ok(Case::Other),
                _ => Err(CaseConversionError),
            },
            _ => Err(CaseConversionError),
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

#[derive(Debug, Error)]
#[error(
    r#"metric type must be a string containing only "ascender", "cap height", "slant height", "x-height", "midHeight", "topHeight", "bodyHeight", "descender", "baseline", or "italic angle""#
)]
pub struct MetricTypeConversionError;

impl TryFrom<Plist> for MetricType {
    type Error = MetricTypeConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::String(s) => match s.as_str() {
                "ascender" => Ok(MetricType::Ascender),
                "baseline" => Ok(MetricType::Baseline),
                "bodyHeight" => Ok(MetricType::BodyHeight),
                "cap height" => Ok(MetricType::CapHeight),
                "descender" => Ok(MetricType::Descender),
                "italic angle" => Ok(MetricType::ItalicAngle),
                "midHeight" => Ok(MetricType::MidHeight),
                "slant height" => Ok(MetricType::SlantHeight),
                "topHeight" => Ok(MetricType::TopHeight),
                "x-height" => Ok(MetricType::XHeight),
                _ => Err(MetricTypeConversionError),
            },
            _ => Err(MetricTypeConversionError),
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

#[derive(Debug, Error)]
#[error(r#"instance type must be a string containing only "variable""#)]
pub struct InstanceTypeConversionError;

impl TryFrom<Plist> for InstanceType {
    type Error = InstanceTypeConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        if let Plist::String(inner) = plist {
            if inner == "variable" {
                return Ok(InstanceType::Variable);
            }
        }
        Err(InstanceTypeConversionError)
    }
}

impl ToPlist for InstanceType {
    fn to_plist(self) -> Plist {
        match self {
            InstanceType::Variable => "variable".to_string().into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ShapeConversionError {
    #[error("shape can only be parsed from a dictionary")]
    WrongVariant,
    #[error("bad component: {0}")]
    BadComponent(Box<GlyphsFromPlistError>),
    #[error("bad path: {0}")]
    BadPath(Box<GlyphsFromPlistError>),
}

impl TryFrom<Plist> for Shape {
    type Error = ShapeConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        match plist {
            Plist::Dictionary(ref dict) => {
                if dict.contains_key("ref") {
                    plist
                        .try_into()
                        .map(Shape::Component)
                        .map_err(Box::new)
                        .map_err(ShapeConversionError::BadComponent)
                } else {
                    plist
                        .try_into()
                        .map(Box::new)
                        .map(Shape::Path)
                        .map_err(Box::new)
                        .map_err(ShapeConversionError::BadPath)
                }
            }
            _ => Err(ShapeConversionError::WrongVariant),
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

#[derive(Debug, Error)]
pub enum CodepointsConversionError {
    #[error("unicode code point must be in the range U+0000–U+10FFFF, got U+{0:04X}")]
    InvalidCodepoint(i64),
    #[error("codepoints can only be parsed from an integer or integer array")]
    WrongVariant,
}

impl TryFrom<Plist> for norad::Codepoints {
    type Error = CodepointsConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let parse_one = |n: i64| {
            let cp: u32 = n
                .try_into()
                .map_err(|_| CodepointsConversionError::InvalidCodepoint(n))?;
            char::try_from(cp).map_err(|_| CodepointsConversionError::InvalidCodepoint(n))
        };
        match plist {
            Plist::Integer(n) => {
                let cp = parse_one(n)?;
                Ok(norad::Codepoints::new([cp]))
            }
            Plist::Array(array) => array
                .into_iter()
                .map(|item| {
                    if let Plist::Integer(n) = item {
                        parse_one(n)
                    } else {
                        Err(CodepointsConversionError::WrongVariant)
                    }
                })
                .collect::<Result<_, _>>(),
            _ => Err(CodepointsConversionError::WrongVariant),
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

#[derive(Debug, Error)]
pub enum NodeConversionError {
    #[error("nodes can only be parsed from an array of length 3")]
    WrongVariant,
    #[error("node without x coordinate")]
    MissingX,
    #[error("node without y coordinate")]
    MissingY,
    #[error("node without type")]
    MissingType,
    #[error("x coordinate must be a float")]
    NotFloatX,
    #[error("y coordinate must be a float")]
    NotFloatY,
    #[error("invalid node attributes: {0}")]
    InvalidAttr(Box<GlyphsFromPlistError>),
    #[error(transparent)]
    InvalidType(#[from] NodeTypeParseError),
}

impl TryFrom<Plist> for Node {
    type Error = NodeConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let Plist::Array(tuple) = plist else {
            return Err(NodeConversionError::WrongVariant);
        };

        let mut tuple_iter = tuple.into_iter();
        let x = tuple_iter
            .next()
            .ok_or(NodeConversionError::MissingX)?
            .as_f64()
            .ok_or(NodeConversionError::NotFloatX)?;
        let y = tuple_iter
            .next()
            .ok_or(NodeConversionError::MissingY)?
            .as_f64()
            .ok_or(NodeConversionError::NotFloatY)?;
        let node_type = tuple_iter
            .next()
            .ok_or(NodeConversionError::MissingType)?
            .try_into()?;
        let attr = tuple_iter
            .next()
            .map(NodeAttrs::try_from)
            .transpose()
            .map_err(Box::new)
            .map_err(NodeConversionError::InvalidAttr)?;

        let pt = Point::new(x, y);
        Ok(Node {
            pt,
            node_type,
            attr,
        })
    }
}

#[derive(Debug, Error)]
#[error(r#"node type must be a string containing only "l", "ls", "c", "cs", "q", "qs", or "o""#)]
pub struct NodeTypeParseError;

impl TryFrom<Plist> for NodeType {
    type Error = NodeTypeParseError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        plist.as_str().ok_or(NodeTypeParseError)?.parse()
    }
}

impl std::str::FromStr for NodeType {
    type Err = NodeTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "l" => Ok(NodeType::Line),
            "ls" => Ok(NodeType::LineSmooth),
            "c" => Ok(NodeType::Curve),
            "cs" => Ok(NodeType::CurveSmooth),
            "q" => Ok(NodeType::QCurve),
            "qs" => Ok(NodeType::QCurveSmooth),
            "o" => Ok(NodeType::OffCurve),
            _ => Err(NodeTypeParseError),
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
        // Construct a tuple of length 3 if there are no attributes, otherwise a
        // tuple of length 4.

        let Node {
            pt,
            node_type,
            attr,
        } = self;

        let mut tuple = vec![
            pt.x.into(),
            pt.y.into(),
            node_type.glyphs_str().to_string().into(),
        ];

        if let Some(attr) = attr {
            tuple.push(attr.to_plist());
        }

        Plist::Array(tuple)
    }
}

#[derive(Debug, Error)]
pub enum PointConversionError {
    #[error("point can only be parsed from an array of length 2")]
    WrongVariant,
    #[error("node without x coordinate")]
    MissingX,
    #[error("node without y coordinate")]
    MissingY,
    #[error("x coordinate must be a float")]
    NotFloatX,
    #[error("y coordinate must be a float")]
    NotFloatY,
}

impl TryFrom<Plist> for Point {
    type Error = PointConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let Plist::Array(tuple) = plist else {
            return Err(PointConversionError::WrongVariant);
        };
        if tuple.len() != 2 {
            return Err(PointConversionError::WrongVariant);
        }

        let mut tuple_iter = tuple.into_iter();
        let x = tuple_iter
            .next()
            .ok_or(PointConversionError::MissingX)?
            .as_f64()
            .ok_or(PointConversionError::NotFloatX)?;
        let y = tuple_iter
            .next()
            .ok_or(PointConversionError::MissingY)?
            .as_f64()
            .ok_or(PointConversionError::NotFloatY)?;
        Ok(Point::new(x, y))
    }
}

impl ToPlist for Point {
    fn to_plist(self) -> Plist {
        Plist::Array(vec![self.x.into(), self.y.into()])
    }
}

#[derive(Debug, Error)]
pub enum ScaleConversionError {
    #[error("scale can only be parsed from an array of length 2")]
    WrongVariant,
    #[error("scale without horizontal value")]
    MissingHorizontal,
    #[error("node without vertical value")]
    MissingVertical,
    #[error("horizontal value must be a float")]
    NotFloatHorizontal,
    #[error("vertical value must be a float")]
    NotFloatVertical,
}

impl TryFrom<Plist> for Scale {
    type Error = ScaleConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let Plist::Array(tuple) = plist else {
            return Err(ScaleConversionError::WrongVariant);
        };
        if tuple.len() != 2 {
            return Err(ScaleConversionError::WrongVariant);
        }

        let mut tuple_iter = tuple.into_iter();
        let horizontal = tuple_iter
            .next()
            .ok_or(ScaleConversionError::MissingHorizontal)?
            .as_f64()
            .ok_or(ScaleConversionError::NotFloatHorizontal)?;
        let vertical = tuple_iter
            .next()
            .ok_or(ScaleConversionError::MissingVertical)?
            .as_f64()
            .ok_or(ScaleConversionError::NotFloatVertical)?;
        Ok(Scale {
            horizontal,
            vertical,
        })
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
        self.nodes.push(Node {
            pt,
            node_type,
            attr: None,
        });
    }

    pub fn rotate_left(&mut self, delta: usize) {
        self.nodes.rotate_left(delta);
    }

    pub fn reverse(&mut self) {
        self.nodes.reverse();
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

#[derive(Debug, Error)]
pub enum KerningConversionError {
    #[error("kerning can only be parsed from a dict[master name, dict[left, dict[right, value]]]")]
    WrongVariant,
    #[error("kerning value for /{left_name}/{right_name} was not a float")]
    NotFloatValue {
        left_name: String,
        right_name: String,
    },
}

impl TryFrom<Plist> for HashMap<String, norad::Kerning> {
    type Error = KerningConversionError;

    fn try_from(plist: Plist) -> Result<Self, Self::Error> {
        let Plist::Dictionary(dict) = plist else {
            return Err(KerningConversionError::WrongVariant);
        };
        dict.into_iter()
            .map(|(master_id, master_kerning)| {
                let Plist::Dictionary(master_kerning) = master_kerning else {
                    return Err(KerningConversionError::WrongVariant);
                };
                let norad_master_kerning = master_kerning
                    .into_iter()
                    .map(|(left, kerns)| {
                        let Plist::Dictionary(kerns) = kerns else {
                            return Err(KerningConversionError::WrongVariant);
                        };
                        let left_name = norad::Name::new(&left).unwrap_or_else(|_| {
                            panic!("glyph name {left:?} valid in Glyphs but not norad")
                        });
                        let norad_kerns = kerns
                            .into_iter()
                            .map(|(right, value)| {
                                let right_name = norad::Name::new(&right).unwrap_or_else(|_| {
                                    panic!("glyph name {right:?} valid in Glyphs but not norad")
                                });
                                let value = value.as_f64().ok_or_else(|| {
                                    KerningConversionError::NotFloatValue {
                                        left_name: left.clone(),
                                        right_name: right.clone(),
                                    }
                                })?;
                                Ok((right_name, value))
                            })
                            .collect::<Result<_, _>>()?;
                        Ok((left_name, norad_kerns))
                    })
                    .collect::<Result<_, _>>()?;
                Ok((master_id, norad_master_kerning))
            })
            .collect::<Result<_, _>>()
    }
}

// TODO: provide field/struct name (context) somehow, especially for errors in dervied code
#[derive(Debug, Error)]
pub enum GlyphsFromPlistError {
    #[error("missing field {0}")]
    MissingField(&'static str),
    #[error("unrecognised fields: {}", .0.join(", "))]
    UnrecognisedFields(Vec<String>),
    #[error("incorrect field type: {0}")]
    Variant(#[from] VariantError),
    #[error(transparent)]
    DownsizeToU16(#[from] DownsizeToU16Error),
    #[error("bad bool: {0}")]
    Bool(#[from] BoolConversionError),
    #[error("bad array: {0}")]
    Array(Box<dyn std::error::Error + Send + Sync>),
    #[error("bad name: {0}")]
    Name(#[from] NameConversionError),
    #[error("bad anchor orientation: {0}")]
    AnchorOrientation(#[from] AnchorOrientationConversionError),
    #[error("bad color: {0}")]
    Color(#[from] ColorConversionError),
    #[error("bad direction: {0}")]
    Direction(#[from] DirectionConversionError),
    #[error("bad case: {0}")]
    Case(#[from] CaseConversionError),
    #[error("bad metric type: {0}")]
    MetricType(#[from] MetricTypeConversionError),
    #[error("bad instance type: {0}")]
    InstanceType(#[from] InstanceTypeConversionError),
    #[error("bad node: {0}")]
    Node(#[from] NodeConversionError),
    #[error("bad point: {0}")]
    Point(#[from] PointConversionError),
    #[error("bad scale: {0}")]
    Scale(#[from] ScaleConversionError),
    #[error("bad shape: {0}")]
    Shape(#[from] ShapeConversionError),
    #[error("bad kerning: {0}")]
    Kerning(#[from] KerningConversionError),
    #[error("bad codepoint(s): {0}")]
    Codepoints(#[from] CodepointsConversionError),
}

impl From<Infallible> for GlyphsFromPlistError {
    fn from(_: Infallible) -> Self {
        unsafe { std::hint::unreachable_unchecked() }
    }
}

impl<E> From<ArrayConversionError<E>> for GlyphsFromPlistError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: ArrayConversionError<E>) -> Self {
        GlyphsFromPlistError::Array(Box::new(err))
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
        assert_eq!(
            Font::load("testdata/NewFontG3.glyphs").unwrap(),
            Default::default()
        );
    }

    #[test]
    fn parse_float_names() {
        let font = Font::load("testdata/FloatNames.glyphs").unwrap();
        println!("{:#?}", font.user_data);
    }

    #[test]
    fn parse_format3_example() {
        let font = match Font::load("testdata/GlyphsFileFormatv3.glyphs") {
            Ok(font) => font,
            Err(why) => panic!("{why}\n{why:?}"),
        };

        assert_eq!(font.app_version, "3259");
        assert_eq!(font.format_version, Some(3));

        assert!(!font.other_stuff.contains_key(".appVersion"));
        assert!(!font.other_stuff.contains_key(".formatVersion"));
    }

    #[test]
    fn roundtrip_plist() {
        let contents = fs::read_to_string("testdata/NewFontG3.glyphs").unwrap();
        let plist = Plist::parse(&contents).unwrap();
        let font: Font = plist.clone().try_into().unwrap();
        let plist_roundtrip = ToPlist::to_plist(font);

        assert_eq!(plist, plist_roundtrip);
    }

    #[test]
    fn only_expected_other_stuff() {
        // TODO: Run on all test fixtures.
        let font = Font::load("testdata/GlyphsFileFormatv3.glyphs").unwrap();

        let other_keys = font.other_stuff.keys().cloned().collect::<HashSet<_>>();

        let disallowed = other_keys
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
            .collect::<HashSet<_>>();

        assert!(disallowed.is_empty());

        // TODO: Implement for nested structs.
    }

    #[test]
    fn error_on_unexpected_fields() {
        #[derive(Debug, FromPlist)]
        struct FooBar {
            _foo: String,
        }

        let with_unexpected = Plist::Dictionary(HashMap::from([
            ("foo".to_owned(), Plist::String("abc".to_owned())),
            ("bar".to_owned(), Plist::String("def".to_owned())),
        ]));

        let err = TryInto::<FooBar>::try_into(with_unexpected)
            .expect_err("shouldn't succeed with unknown fields");
        let GlyphsFromPlistError::UnrecognisedFields(fields) = err else {
            panic!("wrong error variant");
        };
        assert_eq!(fields, vec![String::from("bar")]);
    }

    #[test]
    fn always_assumes_closed() {
        // See: schriftgestalt/GlyphsSDK#92
        // Glyphs assumes that an absent 'closed' attribute means that a path is
        // closed, and so we should ensure that our default value respects this
        // when reading.

        let ambiguous =
            Plist::Dictionary(HashMap::from([("nodes".to_string(), Plist::Array(vec![]))]));

        let path = Path::try_from(ambiguous).unwrap();
        assert!(path.closed);
    }

    #[test]
    fn always_writes_closed() {
        // See: schriftgestalt/GlyphsSDK#92
        // Glyphs always writes the 'closed' attribute, and so we should
        // maintain this behaviour also.

        let path_open = Path::new(false);
        let plist = path_open.to_plist().into_hashmap();
        assert_eq!(plist.get("closed"), Some(&Plist::Integer(0)));

        let path_closed = Path::new(true);
        let plist = path_closed.to_plist().into_hashmap();
        assert_eq!(plist.get("closed"), Some(&Plist::Integer(1)));
    }
}
