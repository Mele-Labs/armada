//! A sparkline: one number's recent values as a thin line, its latest a dot in the accent of its row.
//! No axes and no hover; the value beside it is the reading.

use gpui::{point, px, quad, size, App, BorderStyle, Bounds, Hsla, Pixels, Window};
use gpui_component::plot::scale::{Scale, ScaleLinear};
use gpui_component::plot::shape::{Area, Line};
use gpui_component::plot::{Curve, IntoPlot, Plot};

#[derive(IntoPlot)]
pub struct Spark {
    values: Vec<f64>,
    /// The line's hue, and the surface the end dot is ringed in.
    ink: Hsla,
    surface: Hsla,
}

impl Spark {
    pub fn new(values: Vec<f64>, ink: Hsla, surface: Hsla) -> Spark {
        Spark { values, ink, surface }
    }
}

impl Plot for Spark {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, _cx: &mut App) {
        if self.values.len() < 2 {
            return;
        }
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let pad = 4.;
        let low = self.values.iter().copied().fold(f64::INFINITY, f64::min);
        let high = self.values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // A flat line sits in the middle rather than on the floor.
        let (low, high) = if high - low < 1e-9 { (low - 1., high + 1.) } else { (low, high) };
        let x = ScaleLinear::new([0., (self.values.len() - 1) as f64], [pad, width - pad]);
        let y = ScaleLinear::new([low, high], [height - pad, pad]);
        let placed: Vec<(f32, f32)> = self.values.iter().enumerate().filter_map(|(at, value)| Some((x.tick(&(at as f64))?, y.tick(value)?))).collect();
        let fade = gpui::linear_gradient(180., gpui::linear_color_stop(self.ink.opacity(0.22), 0.), gpui::linear_color_stop(self.ink.opacity(0.), 1.));
        Area::new().data(placed.clone()).x(|(x, _)| Some(*x)).y0(height).y1(|(_, y)| Some(*y)).fill(fade).stroke(gpui::transparent_black()).curve(Curve::Natural).paint(&bounds, window);
        Line::new().data(placed.clone()).x(|(x, _)| Some(*x)).y(|(_, y)| Some(*y)).stroke(self.ink).stroke_width(1.5).curve(Curve::Natural).paint(&bounds, window);
        if let Some((x, y)) = placed.last() {
            let at = bounds.origin + point(px(*x), px(*y));
            window.paint_quad(quad(Bounds::centered_at(at, size(px(7.), px(7.))), px(3.5), self.ink, px(1.5), self.surface, BorderStyle::default()));
        }
    }
}
