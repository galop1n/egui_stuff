//use egui::*;
use egui::{epaint, vec2, Color32, Id, Response, Rounding, Sense, Stroke, Ui, Vec2, Widget};

/// Pie slice.
///
#[derive(Clone, Debug)]
pub struct Slice {
    pub id: Option<Id>,
    pub argument: f64,
    pub width: f64,
    pub value: Option<f64>,
    pub base_offset: Option<f64>,
    pub top_offset: Option<f64>,
    pub name: Option<String>,
    pub fill: Option<Color32>,
    pub stroke: Option<Stroke>,
}

/// Pie chart.
///
pub struct Piechart {
    slices: Vec<Slice>,
    show_background: bool,
    width: Option<f32>,
    height: Option<f32>,
    min_size: Vec2,
    name: Option<String>,
    pub normalization_factor: Option<f64>,
    pub base_offset: Option<f64>,
    pub top_offset: Option<f64>,
    pub fill: Color32,
    pub stroke: Stroke,
    pub element_formatter: Option<Box<dyn Fn(&Slice, &Piechart) -> String>>,
}

impl Slice {
    pub fn new(argument: f64, width: f64) -> Self {
        Self {
            id: None,
            value: None,
            argument,
            width,
            top_offset: None,
            base_offset: None,
            name: None,
            fill: None,
            stroke: None,
        }
    }
    pub fn argument(mut self, argument: f64) -> Self {
        self.argument = argument;
        self
    }
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = Some(name.to_string());
        self
    }
    pub fn id(mut self, id: impl std::hash::Hash) -> Self {
        self.id = Some(Id::new(id));
        self
    }
    pub fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
    pub fn base_offset(mut self, base_offset: f64) -> Self {
        self.base_offset = Some(base_offset);
        self
    }
    pub fn top_offset(mut self, top_offset: f64) -> Self {
        self.top_offset = Some(top_offset);
        self
    }
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self
    }
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }
}

impl Piechart {
    pub fn new(slices: Vec<Slice>) -> Self {
        Self {
            slices,
            name: None,
            show_background: true,
            normalization_factor: None,
            base_offset: None,
            top_offset: None,
            fill: Color32::TRANSPARENT,
            stroke: Stroke::new(1.0, Color32::WHITE),
            element_formatter: None,
            width: None,
            height: None,
            min_size: Vec2::splat(64.0),
        }
    }
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn normalization_factor(mut self, normalization_factor: f64) -> Self {
        self.normalization_factor = Some(normalization_factor);
        self
    }

    pub fn base_offset(mut self, base_offset: f64) -> Self {
        self.base_offset = Some(base_offset);
        self
    }
    pub fn top_offset(mut self, top_offset: f64) -> Self {
        self.top_offset = Some(top_offset);
        self
    }

    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = fill.into();
        self
    }

    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn show_background(mut self, show_background: bool) -> Self {
        self.show_background = show_background;
        self
    }

    /// Width of piechart. By default a piechart will fill the ui it is in.
    pub fn width(mut self, width: f32) -> Self {
        self.min_size.x = width;
        self.width = Some(width);
        self
    }

    /// Height of piechart. By default a piechart will fill the ui it is in.
    pub fn height(mut self, height: f32) -> Self {
        self.min_size.y = height;
        self.height = Some(height);
        self
    }

    /// Minimum size of the piechart view.
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn element_formatter(
        mut self,
        element_formatter: impl Fn(&Slice, &Piechart) -> String + 'static,
    ) -> Self {
        self.element_formatter = Some(Box::new(element_formatter));
        self
    }
}

impl Widget for Piechart {
    fn ui(self, ui: &mut Ui) -> Response {
        use epaint::{Mesh, PathShape, RectShape, Shape, TextureId, Vertex, WHITE_UV};

        let size = ui.available_size().max(self.min_size);
        let width = self.width.unwrap_or(size.x);
        let height = self.height.unwrap_or(size.y);

        let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let painter = ui.painter();

        if self.show_background {
            painter.with_clip_rect(rect).add(RectShape {
                rect,
                rounding: Rounding::same(2.0),
                fill: ui.visuals().extreme_bg_color,
                stroke: ui.visuals().widgets.noninteractive.bg_stroke,
            });
        }

        let spacing = ui.spacing().item_spacing;
        let radius = (rect.width() - 2.0 * spacing.x).min(rect.height() - 2.0 * spacing.y) / 2.0;

        if radius < 1.0 {
            return response;
        }

        let center = rect.center();

        let mouse_pos = response.hover_pos();
        let mouse_angle = mouse_pos
            .map(|p| p - center)
            .map(|v| (-v.y).atan2(v.x))
            .and_then(|mut a| {
                if a < 0.0 {
                    a += 2.0 * std::f32::consts::PI;
                }
                Some(a)
            });
        let mouse_dist = mouse_pos.map(|p| p - center).map(|v| v.length() / radius);

        let normalization_factor =
            self.normalization_factor.map(|f| 1.0 / f).unwrap_or(1.0) * std::f64::consts::PI * 2.0;

        const FULL_CIRCLE_TAP_COUNT: usize = 256;
        let mut vertices: Vec<Vertex> = Vec::with_capacity(2 * FULL_CIRCLE_TAP_COUNT * 3 / 2);
        let mut indices: Vec<u32> = Vec::with_capacity(FULL_CIRCLE_TAP_COUNT * 6 * 3 / 2);
        let mut response_slice_id = None;
        let mut strokes = Vec::with_capacity(self.slices.len());

        for slice in self.slices.iter() {
            let vertex_base_offset = vertices.len() as u32;
            let angle_start = ((slice.argument) * normalization_factor) as f32;
            let angle_end = ((slice.argument + slice.width) * normalization_factor) as f32;

            let base_offset = slice.base_offset.or(self.base_offset).unwrap_or(0.0);
            let top_offset = slice.top_offset.or(self.top_offset).unwrap_or(1.0);
            assert!(base_offset <= top_offset);

            let hovered = if let (Some(angle), Some(dist)) = (mouse_angle, mouse_dist) {
                (angle_start..=angle_end).contains(&angle)
                    && (base_offset..top_offset).contains(&(dist as f64))
            } else {
                false
            };
            if hovered {
                response_slice_id = Some(Id::new(&slice.name));
            }

            let taps = (((angle_end - angle_start) * FULL_CIRCLE_TAP_COUNT as f32
                / (std::f32::consts::PI * 2.0)) as u32)
                .max(2);

            let fill = slice.fill.unwrap_or(self.fill);
            let stroke = slice.stroke.unwrap_or(self.stroke);
            let (fill, stroke) = if hovered {
                (
                    fill.linear_multiply(0.6),
                    Stroke::new(stroke.width, stroke.color.linear_multiply(1.0)),
                )
            } else {
                (
                    fill.linear_multiply(0.2),
                    Stroke::new(stroke.width, stroke.color.linear_multiply(0.4)),
                )
            };

            // [TODO] Solve tesselator spikes and optimize geometry with nil base offset
            let base_offset = (base_offset as f32 * radius).max(1.5);
            let top_offset = top_offset as f32 * radius;

            if fill != Color32::TRANSPARENT {
                let vert = |pos| Vertex {
                    pos,
                    uv: WHITE_UV,
                    color: fill,
                };
                for index in 0..taps {
                    let angle = angle_start as f32
                        + (angle_end - angle_start) * index as f32 / ((taps - 1) as f32);
                    let cos_sin = vec2(angle.cos(), -angle.sin());
                    let pos = center + cos_sin * base_offset;
                    vertices.push(vert(pos));
                    let pos = center + cos_sin * top_offset;
                    vertices.push(vert(pos));
                    if index != taps - 1 {
                        let base = vertex_base_offset + index * 2;
                        const TWO_TRIS: [u32; 6] = [0, 1, 2, 1, 2, 3];
                        indices.extend_from_slice(TWO_TRIS.map(|i| i + base).as_slice());
                    }
                }
            }

            let mut points = Vec::with_capacity(taps as usize * 2);
            for a in (0..taps).into_iter().rev() {
                let angle =
                    angle_start as f32 + (angle_end - angle_start) * a as f32 / ((taps - 1) as f32);
                let cos_sin = vec2(angle.cos(), -angle.sin());
                points.push(center + cos_sin * base_offset);
            }

            for a in 0..taps {
                let angle =
                    angle_start as f32 + (angle_end - angle_start) * a as f32 / ((taps - 1) as f32);
                let angle = angle;
                let c = angle.cos();
                let s: f32 = angle.sin();
                points.push(center + vec2(c, -s) * top_offset);
            }
            if !stroke.is_empty() {
                let pie_slice = PathShape {
                    points,
                    closed: true,
                    fill: Color32::TRANSPARENT,
                    stroke,
                };
                strokes.push(Shape::Path(pie_slice));
            }

            if hovered {
                let label = self
                    .element_formatter
                    .as_deref()
                    .map(|fmt| fmt(&slice, &self))
                    .or_else(|| match (slice.name.as_deref(), slice.value) {
                        (Some(name), Some(value)) => Some(format!("{}: {:.2}", name, value)),
                        (Some(name), _) => Some(format!("{}", name)),
                        (_, Some(value)) => Some(format!("{:.2}", value)),
                        _ => Default::default(),
                    });
                if let Some(label) = label {
                    egui::show_tooltip_at_pointer(ui.ctx(), Id::new("my_tooltip"), |ui| {
                        ui.label(label);
                    });
                }
            }
        }

        painter.add(Mesh {
            indices,
            vertices,
            texture_id: TextureId::Managed(0),
        });
        painter.extend(strokes);
        response.id = response_slice_id.unwrap_or(response.id);
        response
    }
}
