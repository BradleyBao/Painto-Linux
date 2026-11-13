use gtk4::{cairo, gdk};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Computer,
    Pen,
    ObjectEraser,
    PixelEraser,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

struct Stroke {
    points: Vec<Point>,
    color: gdk::RGBA,
    width: f64,
    eraser: bool,
}

pub struct Ink {
    tool: Tool,
    color: gdk::RGBA,
    pen_width: f64,
    eraser_width: f64,
    strokes: Vec<Stroke>,
    active: Option<Stroke>,
}

impl Default for Ink {
    fn default() -> Self {
        Self {
            tool: Tool::Computer,
            color: gdk::RGBA::BLACK,
            pen_width: 5.0,
            eraser_width: 30.0,
            strokes: Vec::new(),
            active: None,
        }
    }
}

impl Ink {
    pub fn set_tool(&mut self, tool: Tool) {
        self.finish();
        self.tool = tool;
    }

    pub fn set_color(&mut self, color: gdk::RGBA) {
        self.color = color;
    }

    pub fn set_pen_width(&mut self, width: f64) {
        self.pen_width = width.clamp(1.0, 40.0);
    }

    pub fn clear(&mut self) {
        self.active = None;
        self.strokes.clear();
    }

    pub fn begin(&mut self, x: f64, y: f64) {
        let point = Point::new(x, y);
        match self.tool {
            Tool::Pen | Tool::PixelEraser => {
                let eraser = self.tool == Tool::PixelEraser;
                self.active = Some(Stroke {
                    points: vec![point],
                    color: self.color,
                    width: if eraser {
                        self.eraser_width
                    } else {
                        self.pen_width
                    },
                    eraser,
                });
            }
            Tool::ObjectEraser => {
                self.erase_at(point);
            }
            Tool::Computer => {}
        }
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        let point = Point::new(x, y);
        if self.tool == Tool::ObjectEraser {
            self.erase_at(point);
        } else if let Some(stroke) = &mut self.active
            && stroke.points.last().copied() != Some(point)
        {
            stroke.points.push(point);
        }
    }

    pub fn finish(&mut self) {
        if let Some(stroke) = self.active.take() {
            self.strokes.push(stroke);
        }
    }

    fn erase_at(&mut self, point: Point) {
        let radius = self.eraser_width / 2.0;
        if let Some(index) = self.strokes.iter().rposition(|stroke| {
            !stroke.eraser
                && stroke
                    .points
                    .windows(2)
                    .any(|segment| distance_to_segment(point, segment[0], segment[1]) < radius)
                || !stroke.eraser
                    && stroke.points.len() == 1
                    && distance(point, stroke.points[0]) < radius
        }) {
            self.strokes.remove(index);
        }
    }

    pub fn draw(&self, context: &cairo::Context) -> Result<(), cairo::Error> {
        context.push_group();
        for stroke in self.strokes.iter().chain(self.active.iter()) {
            context.set_operator(if stroke.eraser {
                cairo::Operator::Clear
            } else {
                cairo::Operator::Over
            });
            draw_stroke(context, stroke)?;
        }
        context.pop_group_to_source()?;
        context.set_operator(cairo::Operator::Over);
        context.paint()
    }
}

fn draw_stroke(context: &cairo::Context, stroke: &Stroke) -> Result<(), cairo::Error> {
    let Some(first) = stroke.points.first() else {
        return Ok(());
    };
    context.set_source_rgba(
        stroke.color.red().into(),
        stroke.color.green().into(),
        stroke.color.blue().into(),
        stroke.color.alpha().into(),
    );
    context.set_line_width(stroke.width);
    context.set_line_cap(cairo::LineCap::Round);
    context.set_line_join(cairo::LineJoin::Round);
    if stroke.points.len() == 1 {
        context.arc(
            first.x,
            first.y,
            stroke.width / 2.0,
            0.0,
            std::f64::consts::TAU,
        );
        context.fill()
    } else {
        context.move_to(first.x, first.y);
        for point in stroke.points.iter().skip(1) {
            context.line_to(point.x, point.y);
        }
        context.stroke()
    }
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        return distance(point, start);
    }
    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    distance(point, Point::new(start.x + t * dx, start.y + t * dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_click_creates_a_dot() {
        let mut ink = Ink::default();
        ink.set_tool(Tool::Pen);
        ink.begin(20.0, 30.0);
        ink.finish();
        assert_eq!(ink.strokes.len(), 1);
        assert_eq!(ink.strokes[0].points, vec![Point::new(20.0, 30.0)]);
    }

    #[test]
    fn object_eraser_hits_between_samples_and_removes_top_stroke() {
        let mut ink = Ink::default();
        ink.set_tool(Tool::Pen);
        for _ in 0..2 {
            ink.begin(0.0, 0.0);
            ink.move_to(100.0, 0.0);
            ink.finish();
        }
        ink.set_tool(Tool::ObjectEraser);
        ink.begin(50.0, 3.0);
        assert_eq!(ink.strokes.len(), 1);
    }

    #[test]
    fn pixel_eraser_only_clears_ink_layer() {
        let mut ink = Ink::default();
        ink.set_tool(Tool::Pen);
        ink.begin(5.0, 10.0);
        ink.move_to(45.0, 10.0);
        ink.finish();
        ink.set_tool(Tool::PixelEraser);
        ink.begin(25.0, 10.0);
        ink.finish();

        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 50, 20).unwrap();
        let context = cairo::Context::new(&surface).unwrap();
        ink.draw(&context).unwrap();
        drop(context);
        surface.flush();
        let data = surface.data().unwrap();
        let alpha_at = |x: usize| data[(10 * 50 + x) * 4 + 3];
        assert!(alpha_at(5) > 0);
        assert_eq!(alpha_at(25), 0);
    }
}
