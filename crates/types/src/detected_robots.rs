use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct DetectedRobots {
    pub robot_positions: Vec<Vector2<f32>>,
}

fn map_to_zero_one(x: &f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScaledBoxes(pub [Box; 4]);

impl ScaledBoxes {
    pub fn from_output(
        values: &[f32; 4 * 6],
        grid_position: Vector2<f32>,
        grid_size: Vector2<f32>,
        camera_image_size: Vector2<f32>,
        box_scalings: &[Vector2<f32>; 4],
    ) -> Self {
        // assert_eq!(values.len(), 4 * 6);
        let values: Vec<_> = values.iter().map(map_to_zero_one).collect();
        Self([
            Box::from_data(
                Vector2::new(values[0], values[1]),
                Vector2::new(values[2], values[3]),
                values[4],
                values[5],
                grid_position,
                grid_size,
                camera_image_size,
                box_scalings[0],
            ),
            Box::from_data(
                Vector2::new(values[6], values[7]),
                Vector2::new(values[8], values[9]),
                values[10],
                values[11],
                grid_position,
                grid_size,
                camera_image_size,
                box_scalings[1],
            ),
            Box::from_data(
                Vector2::new(values[12], values[13]),
                Vector2::new(values[14], values[15]),
                values[16],
                values[17],
                grid_position,
                grid_size,
                camera_image_size,
                box_scalings[2],
            ),
            Box::from_data(
                Vector2::new(values[18], values[19]),
                Vector2::new(values[20], values[21]),
                values[22],
                values[23],
                grid_position,
                grid_size,
                camera_image_size,
                box_scalings[3],
            ),
        ])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Box {
    pub center: Point2<f32>,
    pub size: Vector2<f32>,
    pub probability: f32,
    pub distance: f32,
}

impl Box {
    fn from_data(
        center: Vector2<f32>,
        size: Vector2<f32>,
        probability: f32,
        distance: f32,
        grid_position: Vector2<f32>,
        grid_size: Vector2<f32>,
        camera_image_size: Vector2<f32>,
        scaling: Vector2<f32>,
    ) -> Self {
        Self {
            center: (center + grid_position)
                .component_div(&grid_size)
                .component_mul(&camera_image_size)
                .into(),
            size: (size * 10.0)
                .component_mul(&scaling)
                .component_mul(&camera_image_size)
                .component_div(&grid_size),
            probability,
            distance: distance * 10.0,
        }
    }
}
