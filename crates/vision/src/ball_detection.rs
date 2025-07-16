use color_eyre::Result;
use projection::camera_matrix::CameraMatrix;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use types::{
    ball_detection::BallPercept, perspective_grid_candidates::PerspectiveGridCandidates,
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct BallDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    _camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    _perspective_grid_candidates:
        RequiredInput<Option<PerspectiveGridCandidates>, "perspective_grid_candidates?">,
    _image: Input<YCbCr422Image, "image">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub balls: MainOutput<Option<Vec<BallPercept>>>,
}

impl BallDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs {
            balls: Some(Vec::new()).into(),
        })
    }
}
