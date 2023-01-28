use std::{num::NonZeroU32, path::PathBuf};

use color_eyre::Result;
use compiled_nn::CompiledNN;
use context_attribute::context;
use fast_image_resize::Resizer;
use framework::{AdditionalOutput, MainOutput};
// use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, Luma};
use nalgebra::Vector2;
use types::{image::Image, DetectedRobots, ScaledBoxes};

use crate::CyclerInstance;

pub struct RobotDetection {
    neural_network: CompiledNN,
}

#[context]
pub struct CreationContext {
    pub neural_network_path: Parameter<PathBuf, "robot_detection.$cycler_instance.neural_network">,
}

#[context]
pub struct CycleContext {
    pub image: Input<Image, "image">,
    pub instance: CyclerInstance,
    pub boxes: AdditionalOutput<Vec<ScaledBoxes>, "robot_boxes">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_robots: MainOutput<Option<DetectedRobots>>,
}

const NUMBER_OF_SCALINGS: usize = 4;
const FLOATS_PER_BOX: usize = 6;

impl RobotDetection {
    pub fn new(context: CreationContext) -> Result<Self> {
        let mut neural_network = CompiledNN::default();
        neural_network.compile(&context.neural_network_path);
        Ok(Self { neural_network })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if let CyclerInstance::VisionBottom = context.instance {
            return Ok(MainOutputs::default());
        }

        let grayscale_camera_image = create_grayscale_image(context.image);

        let source_image = fast_image_resize::Image::from_vec_u8(
            NonZeroU32::new(640).unwrap(),
            NonZeroU32::new(480).unwrap(),
            grayscale_camera_image,
            fast_image_resize::PixelType::U8,
        )
        .unwrap();
        let input_layer = self.neural_network.input_mut(0);
        let small_image_width = input_layer.dimensions[1] as usize;
        let small_image_height = input_layer.dimensions[0] as usize;

        let mut small_image = fast_image_resize::Image::new(
            NonZeroU32::new(small_image_width as u32).unwrap(),
            NonZeroU32::new(small_image_height as u32).unwrap(),
            source_image.pixel_type(),
        );
        resize_image(&source_image, &mut small_image);

        // let file = File::create(format!(
        //     "./test_{}.png",
        //     SystemTime::now()
        //         .duration_since(UNIX_EPOCH)
        //         .unwrap()
        //         .as_millis()
        // ))
        // .unwrap();
        // let mut result_buf = BufWriter::new(file);
        // // let image_buffer = image::ImageBuffer::<Luma<u8>, _>::from_raw(
        // //     small_image_width as u32,
        // //     small_image_height as u32,
        // //     small_image.buffer(),
        // // );
        // // image_buffer.sav
        // PngEncoder::new(&mut result_buf)
        //     .write_image(
        //         small_image.buffer(),
        //         small_image_width as u32,
        //         small_image_height as u32,
        //         ColorType::L8,
        //     )
        //     .unwrap();
        copy_into_tensor(
            &small_image,
            small_image_height,
            small_image_width,
            input_layer.data,
        );

        self.neural_network.apply();

        let camera_image_width = 640.0;
        let camera_image_height = 480.0;
        let camera_image_size = Vector2::new(camera_image_width, camera_image_height);

        let grid_boxes = create_boxes(&mut self.neural_network, camera_image_size);
        context.boxes.fill_if_subscribed(move || grid_boxes);

        let robot_positions = Vec::new();
        Ok(MainOutputs {
            detected_robots: Some(DetectedRobots { robot_positions }).into(),
        })
    }
}

fn create_boxes(
    neural_network: &mut CompiledNN,
    camera_image_size: Vector2<f32>,
) -> Vec<ScaledBoxes> {
    let output_layer = neural_network.output(0);

    let box_scalings = [
        Vector2::new(0.5, 1.0),
        Vector2::new(1.0, 2.0),
        Vector2::new(2.0, 4.0),
        Vector2::new(3.0, 6.0),
    ];
    let object_threshold = 0.6_f32;
    // let threshold = -1.0 * (1.0 / object_threshold - 1.0).ln();
    let grid_height = output_layer.dimensions[0] as usize;
    let grid_width = output_layer.dimensions[1] as usize;
    let grid_size = Vector2::new(grid_width as f32, grid_height as f32);
    let mut grid_boxes = Vec::new();
    for y in 0..grid_height {
        for x in 0..grid_width {
            let grid_position = Vector2::new(x as f32, y as f32);
            let data_offset = (y * grid_width + x) * NUMBER_OF_SCALINGS * FLOATS_PER_BOX;
            let data_slice =
                &output_layer.data[data_offset..data_offset + NUMBER_OF_SCALINGS * FLOATS_PER_BOX];
            let boxes = ScaledBoxes::from_output(
                data_slice.try_into().unwrap(),
                grid_position,
                grid_size,
                camera_image_size,
                &box_scalings,
            );
            grid_boxes.push(boxes);
        }
    }
    grid_boxes
}

fn create_grayscale_image(camera_image: &Image) -> Vec<u8> {
    let mut grayscale_camera_image = Vec::with_capacity(640 * 480);
    for i in 0..640 * 480 {
        let ptr = camera_image.buffer.as_ptr() as *const u8;
        grayscale_camera_image.push(unsafe { *ptr.offset(2 * i as isize) });
    }
    grayscale_camera_image
}

fn resize_image(
    source_image: &fast_image_resize::Image,
    destination_image: &mut fast_image_resize::Image,
) {
    let mut resizer = Resizer::new(fast_image_resize::ResizeAlg::Convolution(
        fast_image_resize::FilterType::Box,
    ));
    resizer
        .resize(&source_image.view(), &mut destination_image.view_mut())
        .unwrap();
}

fn copy_into_tensor(
    small_image: &fast_image_resize::Image,
    small_image_height: usize,
    small_image_width: usize,
    input_layer: &mut [f32],
) {
    for y in 0..small_image_height {
        for x in 0..small_image_width {
            input_layer[x + y * small_image_width] =
                small_image.buffer()[x + y * small_image_width] as f32;
        }
    }
}
