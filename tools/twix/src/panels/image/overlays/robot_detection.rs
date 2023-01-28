use std::str::FromStr;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use eframe::epaint::{Color32, Stroke};
use types::{Ball, CandidateEvaluation, ScaledBoxes};

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RobotDetection {
    boxes: ValueBuffer,
}

impl Overlay for RobotDetection {
    const NAME: &'static str = "Robot Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        let camera_position = match selected_cycler {
            Cycler::VisionTop => "top",
            Cycler::VisionBottom => "bottom",
            cycler => panic!("Invalid vision cycler: {cycler}"),
        };
        Self {
            boxes: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.additional.robot_boxes", selected_cycler))
                    .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let boxes: Vec<ScaledBoxes> = self.boxes.require_latest()?;
        for scaled_box in &boxes {
            for robot_box in &scaled_box.0 {
                let color = if robot_box.probability > 0.7 {
                    Color32::BLUE
                } else {
                    continue;
                    Color32::YELLOW
                };
                let line_stroke = Stroke::new(2.0, color);
                painter.rect_stroke(
                    robot_box.center - robot_box.size / 2.0,
                    robot_box.center + robot_box.size / 2.0,
                    line_stroke,
                );
            }
        }

        Ok(())
    }
}
