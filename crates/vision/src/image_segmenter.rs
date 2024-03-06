use std::time::{Duration, Instant};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{point, Isometry2, Transform};
use types::{
    camera_matrix::CameraMatrix,
    color::{Intensity, Rgb, RgbChannel, YCbCr444},
    field_color::FieldColor,
    horizon::Horizon,
    image_segments::{EdgeType, ImageSegments, ScanGrid, ScanLine, Segment},
    interpolated::Interpolated,
    limb::{is_above_limbs, Limb, ProjectedLimbs},
    parameters::{EdgeDetectionSourceParameters, MedianModeParameters},
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct ImageSegmenter {
    ground_to_field_of_home_after_coin_toss_before_second_half: Isometry2<Ground, Field>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    image_segmenter_cycle_time: AdditionalOutput<Duration, "image_segmenter_cycle_time">,

    image: Input<YCbCr422Image, "image">,

    camera_matrix: Input<Option<CameraMatrix>, "camera_matrix?">,
    ground_to_field_of_home_after_coin_toss_before_second_half: Input<
        Option<Isometry2<Ground, Field>>,
        "Control",
        "ground_to_field_of_home_after_coin_toss_before_second_half?",
    >,
    field_color: Input<FieldColor, "field_color">,
    projected_limbs: Input<Option<ProjectedLimbs>, "projected_limbs?">,

    horizontal_stride: Parameter<usize, "image_segmenter.$cycler_instance.horizontal_stride">,
    vertical_stride: Parameter<usize, "image_segmenter.$cycler_instance.vertical_stride">,
    vertical_edge_detection_source: Parameter<
        EdgeDetectionSourceParameters,
        "image_segmenter.$cycler_instance.vertical_edge_detection_source",
    >,
    vertical_edge_threshold:
        Parameter<Interpolated, "image_segmenter.$cycler_instance.vertical_edge_threshold">,
    vertical_median_mode:
        Parameter<MedianModeParameters, "image_segmenter.$cycler_instance.vertical_median_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image_segments: MainOutput<ImageSegments>,
}

impl ImageSegmenter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            ground_to_field_of_home_after_coin_toss_before_second_half: Transform::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if let Some(ground_to_field_of_home_after_coin_toss_before_second_half) =
            context.ground_to_field_of_home_after_coin_toss_before_second_half
        {
            self.ground_to_field_of_home_after_coin_toss_before_second_half =
                *ground_to_field_of_home_after_coin_toss_before_second_half;
        }

        let begin = Instant::now();
        let projected_limbs = context
            .projected_limbs
            .map_or(Default::default(), |projected_limbs| {
                projected_limbs.limbs.as_slice()
            });

        let horizon = context
            .camera_matrix
            .map_or(Horizon::default(), |camera_matrix| camera_matrix.horizon);
        let scan_grid = new_grid(
            context.image,
            &horizon,
            context.field_color,
            *context.horizontal_stride,
            *context.vertical_stride,
            *context.vertical_edge_detection_source,
            context
                .vertical_edge_threshold
                .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half)
                as i16,
            *context.vertical_median_mode,
            projected_limbs,
        );
        let end = Instant::now();
        context
            .image_segmenter_cycle_time
            .fill_if_subscribed(|| end - begin);
        Ok(MainOutputs {
            image_segments: ImageSegments { scan_grid }.into(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn new_grid(
    image: &YCbCr422Image,
    horizon: &Horizon,
    field_color: &FieldColor,
    horizontal_stride: usize,
    vertical_stride: usize,
    vertical_edge_detection_source: EdgeDetectionSourceParameters,
    vertical_edge_threshold: i16,
    vertical_median_mode: MedianModeParameters,
    projected_limbs: &[Limb],
) -> ScanGrid {
    let horizon_y_minimum = horizon
        .horizon_y_minimum()
        .clamp(0.0, image.height() as f32);

    ScanGrid {
        vertical_scan_lines: (2..image.width() - 2)
            .step_by(horizontal_stride)
            .map(|x| {
                new_vertical_scan_line(
                    image,
                    field_color,
                    x,
                    vertical_stride,
                    vertical_edge_detection_source,
                    vertical_edge_threshold,
                    vertical_median_mode,
                    horizon_y_minimum,
                    projected_limbs,
                )
            })
            .collect(),
    }
}

struct ScanLineState {
    previous_value: i16,
    previous_difference: i16,
    maximum_difference: i16,
    maximum_difference_position: u16,
    start_position: u16,
    start_edge_type: EdgeType,
}

impl ScanLineState {
    fn new(previous_value: i16, start_position: u16, start_edge_type: EdgeType) -> Self {
        Self {
            previous_value,
            previous_difference: Default::default(),
            maximum_difference: Default::default(),
            maximum_difference_position: Default::default(),
            start_position,
            start_edge_type,
        }
    }
}

fn median_of_three(values: [u8; 3]) -> u8 {
    let [first, second, third] = values;
    // TODO: replace with same approach as median_of_five()
    if first <= second {
        if second <= third {
            // first <= second <= third
            second
        } else if first <= third {
            // first <= third < second
            third
        } else {
            // third < first <= second
            first
        }
    } else if first <= third {
        // second < first <= third
        first
    } else if second <= third {
        // second <= third < first
        third
    } else {
        // third < second <= first
        second
    }
}

fn median_of_five(mut values: [u8; 5]) -> u8 {
    let (_, median, _) = values.select_nth_unstable(2);
    *median
}

#[allow(clippy::too_many_arguments)]
fn new_vertical_scan_line(
    image: &YCbCr422Image,
    field_color: &FieldColor,
    position: u32,
    stride: usize,
    edge_detection_source: EdgeDetectionSourceParameters,
    edge_threshold: i16,
    median_mode: MedianModeParameters,
    horizon_y_minimum: f32,
    projected_limbs: &[Limb],
) -> ScanLine {
    let (start_y, end_y) = match median_mode {
        MedianModeParameters::Disabled => (horizon_y_minimum as u32, image.height()),
        MedianModeParameters::ThreePixels => ((horizon_y_minimum as u32) + 1, image.height() - 1),
        MedianModeParameters::FivePixels => ((horizon_y_minimum as u32) + 2, image.height() - 2),
    };
    if start_y >= end_y {
        return ScanLine {
            position: position as u16,
            segments: Vec::new(),
        };
    }

    let edge_detection_value =
        edge_detection_value_at(position, start_y, image, edge_detection_source, median_mode);
    let mut state = ScanLineState::new(
        edge_detection_value,
        horizon_y_minimum as u16,
        EdgeType::ImageBorder,
    );

    let mut segments = Vec::with_capacity((end_y - start_y) as usize / stride);
    for y in (start_y..end_y).step_by(stride) {
        let edge_detection_value =
            edge_detection_value_at(position, y, image, edge_detection_source, median_mode);

        if let Some(segment) =
            detect_edge(&mut state, y as u16, edge_detection_value, edge_threshold)
        {
            if segment_is_below_limbs(position as u16, &segment, projected_limbs) {
                fix_previous_edge_type(&mut segments);
                break;
            }
            segments.push(set_color_in_vertical_segment(
                segment,
                image,
                position,
                field_color,
            ));
        }
    }

    let last_segment = Segment {
        start: state.start_position,
        end: image.height() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::ImageBorder,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    if !segment_is_below_limbs(position as u16, &last_segment, projected_limbs) {
        segments.push(set_color_in_vertical_segment(
            last_segment,
            image,
            position,
            field_color,
        ));
    }

    ScanLine {
        position: position as u16,
        segments,
    }
}

fn edge_detection_value_at(
    x: u32,
    y: u32,
    image: &YCbCr422Image,
    edge_detection_source: EdgeDetectionSourceParameters,
    median_mode: MedianModeParameters,
) -> i16 {
    let pixel = pixel_to_edge_detection_value(image.at(x, y), edge_detection_source);

    (match median_mode {
        MedianModeParameters::Disabled => pixel,
        MedianModeParameters::ThreePixels => {
            let pixels = [image.at(x - 1, y), image.at(x, y), image.at(x + 1, y)]
                .map(|pixel| pixel_to_edge_detection_value(pixel, edge_detection_source));
            median_of_three(pixels)
        }
        MedianModeParameters::FivePixels => {
            let pixels = [
                image.at(x - 2, y),
                image.at(x - 1, y),
                image.at(x, y),
                image.at(x + 1, y),
                image.at(x + 2, y),
            ]
            .map(|pixel| pixel_to_edge_detection_value(pixel, edge_detection_source));
            median_of_five(pixels)
        }
    } as i16)
}

fn pixel_to_edge_detection_value(
    pixel: YCbCr444,
    edge_detection_source: EdgeDetectionSourceParameters,
) -> u8 {
    match edge_detection_source {
        EdgeDetectionSourceParameters::Luminance => pixel.y,
        EdgeDetectionSourceParameters::GreenChromaticity => {
            let rgb = Rgb::from(pixel);
            (rgb.get_chromaticity(RgbChannel::Green) * 255.0) as u8
        }
    }
}

fn set_color_in_vertical_segment(
    mut segment: Segment,
    image: &YCbCr422Image,
    x: u32,
    field_color: &FieldColor,
) -> Segment {
    segment.color = match segment.length() {
        6.. => {
            let length = segment.length();
            let first_position = segment.start + (length / 6);
            let second_position = segment.start + ((length * 2) / 6);
            let third_position = segment.start + ((length * 3) / 6);
            let fourth_position = segment.start + ((length * 4) / 6);
            let fifth_position = segment.start + ((length * 5) / 6);

            let first_pixel = image.at(x, first_position as u32);
            let second_pixel = image.at(x, second_position as u32);
            let third_pixel = image.at(x, third_position as u32);
            let fourth_pixel = image.at(x, fourth_position as u32);
            let fifth_pixel = image.at(x, fifth_position as u32);

            let y = median_of_five([
                first_pixel.y,
                second_pixel.y,
                third_pixel.y,
                fourth_pixel.y,
                fifth_pixel.y,
            ]);
            let cb = median_of_five([
                first_pixel.cb,
                second_pixel.cb,
                third_pixel.cb,
                fourth_pixel.cb,
                fifth_pixel.cb,
            ]);
            let cr = median_of_five([
                first_pixel.cr,
                second_pixel.cr,
                third_pixel.cr,
                fourth_pixel.cr,
                fifth_pixel.cr,
            ]);

            YCbCr444::new(y, cb, cr)
        }
        4..=5 => {
            let length = segment.length();
            let first_position = segment.start + (length / 4);
            let second_position = segment.start + ((length * 2) / 4);
            let third_position = segment.start + ((length * 3) / 4);

            let first_pixel = image.at(x, first_position as u32);
            let second_pixel = image.at(x, second_position as u32);
            let third_pixel = image.at(x, third_position as u32);

            let y = median_of_three([first_pixel.y, second_pixel.y, third_pixel.y]);
            let cb = median_of_three([first_pixel.cb, second_pixel.cb, third_pixel.cb]);
            let cr = median_of_three([first_pixel.cr, second_pixel.cr, third_pixel.cr]);

            YCbCr444::new(y, cb, cr)
        }
        0..=3 => {
            let position = segment.start + segment.length() / 2;
            image.at(x, position as u32)
        }
    };
    segment.color = if segment.length() >= 4 {
        let spacing = segment.length() / 4;
        let first_position = segment.start + spacing;
        let second_position = segment.start + 2 * spacing;
        let third_position = segment.start + 3 * spacing;

        let first_pixel = image.at(x, first_position as u32);
        let second_pixel = image.at(x, second_position as u32);
        let third_pixel = image.at(x, third_position as u32);

        let y = median_of_three([first_pixel.y, second_pixel.y, third_pixel.y]);
        let cb = median_of_three([first_pixel.cb, second_pixel.cb, third_pixel.cb]);
        let cr = median_of_three([first_pixel.cr, second_pixel.cr, third_pixel.cr]);
        YCbCr444::new(y, cb, cr)
    } else {
        let position = segment.start + segment.length() / 2;
        image.at(x, position as u32)
    };
    segment.field_color = field_color.get_intensity(segment.color);
    segment
}

fn segment_is_below_limbs(
    scan_line_position: u16,
    segment: &Segment,
    projected_limbs: &[Limb],
) -> bool {
    !is_above_limbs(
        point![scan_line_position as f32, segment.end as f32],
        projected_limbs,
    )
}

fn fix_previous_edge_type(segments: &mut [Segment]) {
    if let Some(previous_segment) = segments.last_mut() {
        previous_segment.end_edge_type = EdgeType::LimbBorder;
    }
}

fn detect_edge(
    state: &mut ScanLineState,
    position: u16,
    value: i16,
    edge_threshold: i16,
) -> Option<Segment> {
    let value_difference = value - state.previous_value;

    let differences_have_initial_values = state.maximum_difference == 0 && value_difference == 0;
    let new_difference_is_more_positive =
        state.maximum_difference >= 0 && value_difference >= state.maximum_difference;
    let new_difference_is_more_negative =
        state.maximum_difference <= 0 && value_difference <= state.maximum_difference;

    if value_difference.abs() >= edge_threshold
        && (differences_have_initial_values
            || new_difference_is_more_positive
            || new_difference_is_more_negative)
    {
        state.maximum_difference = value_difference;
        state.maximum_difference_position = position;
    }

    let found_rising_edge =
        state.previous_difference >= edge_threshold && value_difference < edge_threshold;
    let found_falling_edge =
        state.previous_difference <= -edge_threshold && value_difference > -edge_threshold;

    let segment = if found_rising_edge || found_falling_edge {
        let end_edge_type = if found_rising_edge {
            EdgeType::Rising
        } else {
            EdgeType::Falling
        };
        let segment = Segment {
            start: state.start_position,
            end: state.maximum_difference_position,
            start_edge_type: state.start_edge_type,
            end_edge_type,
            color: Default::default(),
            field_color: Intensity::Low,
        };
        state.maximum_difference = 0;
        state.start_position = state.maximum_difference_position;
        state.start_edge_type = end_edge_type;

        Some(segment)
    } else {
        None
    };

    state.previous_value = value;
    state.previous_difference = value_difference;

    segment
}

#[cfg(test)]
mod tests {
    use itertools::iproduct;
    use types::color::YCbCr422;

    use super::*;

    #[test]
    fn maximum_with_sign_switch() {
        let image = YCbCr422Image::load_from_444_png(
            "../../tests/data/white_wall_with_a_little_desk_in_front.png",
        )
        .unwrap();
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };
        let vertical_stride = 2;
        let vertical_edge_threshold = 16;
        let vertical_median_mode = MedianModeParameters::Disabled;
        let vertical_edge_detection_source = EdgeDetectionSourceParameters::Luminance;
        let horizon_y_minimum = 0.0;
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            12,
            vertical_stride,
            vertical_edge_detection_source,
            vertical_edge_threshold,
            vertical_median_mode,
            horizon_y_minimum,
            &[],
        );
        assert_eq!(scan_line.position, 12);
        assert!(scan_line.segments.len() >= 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert!(scan_line.segments[0].length() >= 255);
        assert!(scan_line.segments[0].length() <= 270);
        assert!(scan_line.segments[1].length() >= 45);
        assert!(scan_line.segments[1].length() <= 55);
    }

    #[test]
    fn image_with_one_vertical_segment_without_median() {
        let image = YCbCr422Image::zero(6, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_with_one_vertical_segment_with_median() {
        let image = YCbCr422Image::zero(6, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_vertical_color_three_pixels() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            3,
            vec![
                // only evaluating every second 422 pixel
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 15, 14, 13),
                YCbCr422::new(0, 10, 10, 10),
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 15);
        assert_eq!(scan_line.segments[0].color.cr, 13);
    }

    #[test]
    fn image_vertical_color_twelve_pixels() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            12,
            vec![
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 1, 1, 1),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 1, 1, 1),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 2, 2, 2),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 1);
        assert_eq!(scan_line.segments[0].color.cr, 1);
    }

    #[test]
    fn image_with_three_vertical_increasing_segments_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            12,
            vec![
                // only evaluating every secondth pixel
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped

                                           // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0  0     0          0                   1             0
        // 1  1     0          0                   0             0
        // 1  0     1          1                   1             1 -> end segment at position 2
        // 2  1     0          0                   0             0
        // 2  0     1          1                   1             1 -> end segment at position 6
        // 3  1     0          0                   0             0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
    }

    #[test]
    fn image_with_three_vertical_increasing_segments_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            12,
            [
                // only evaluating every second pixel
                row(0),
                row(0), // skipped
                row(1),
                // segment boundary will be here
                row(1), // skipped
                row(1),
                row(1), // skipped
                row(2),
                // segment boundary will be here
                row(2), // skipped
                row(2),
                row(2), // skipped
                row(3),
                row(3), // skipped
                        // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        // y  y_median  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0
        // 0  0         0     0          0                   1             0
        // 1
        // 1  1         1     0          0                   0             0
        // 1
        // 1  1         0     1          1                   1             1 -> end segment at position 3
        // 2
        // 2  2         1     0          0                   0             0
        // 2
        // 2  2         0     1          1                   1             1 -> end segment at position 7
        // 3
        // 3
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 3);
        assert_eq!(scan_line.segments[1].end, 7);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].start, 7);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_decreasing_segments_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            12,
            vec![
                // only evaluating every secondth 422 pixel
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped

                                           // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3   0     0         0                    1              0
        // 2  -1     0         0                    0              0
        // 2   0    -1         1                    1              1 -> end segment at position 2
        // 1  -1     0         0                    0              0
        // 1   0    -1         1                    1              1 -> end segment at position 6
        // 0  -1     0         0                    0              0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_decreasing_segments_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            12,
            [
                // only evaluating every secondth 422 pixel
                row(3),
                row(3), // skipped
                row(2),
                // segment boundary will be here
                row(2), // skipped
                row(2),
                row(2), // skipped
                row(1),
                // segment boundary will be here
                row(1), // skipped
                row(1),
                row(1), // skipped
                row(0),
                row(0), // skipped

                        // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        // y  y_median  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3
        // 3  3   0     0         0                    1              0
        // 2
        // 2  2  -1     0         0                    0              0
        // 2
        // 2  2   0    -1         1                    1              1 -> end segment at position 3
        // 1
        // 1  1  -1     0         0                    0              0
        // 1
        // 1  1   0    -1         1                    1              1 -> end segment at position 7
        // 0
        // 0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 3);
        assert_eq!(scan_line.segments[1].end, 7);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 7);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_segments_with_higher_differences_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            44,
            vec![
                // only evaluating every secondth 422 pixel
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                YCbCr422::new(4, 0, 0, 0),
                YCbCr422::new(4, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(5, 0, 0, 0),
                YCbCr422::new(5, 0, 0, 0), // skipped
                YCbCr422::new(4, 0, 0, 0),
                YCbCr422::new(4, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped

                                           // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 1   1     0         0                   0             0
        // 2   1     1         1                   0             0
        // 3   1     1         1                   0             0
        // 4   1     1         1                   0             0
        // 5   1     1         1                   0             0
        // 4  -1     1         1                   1             1 -> end segment at position 16
        // 3  -1    -1         0                   1             0
        // 2  -1    -1         0                   1             0
        // 1  -1    -1         0                   1             0
        // 0  -1    -1         0                   1             0
        // 0   0    -1         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // -> end segment at position 44

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 1   1     0         0                    1              0
        // 2   1     1         0                    1              0
        // 3   1     1         0                    1              0
        // 4   1     1         0                    1              0
        // 5   1     1         0                    1              0
        // 4  -1     1         0                    0              0
        // 3  -1    -1         1                    0              0
        // 2  -1    -1         1                    0              0
        // 1  -1    -1         1                    0              0
        // 0  -1    -1         1                    0              0
        // 0   0    -1         1                    1              1 -> end segment at position 26
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // -> end segment at position 44

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 16);
        assert_eq!(scan_line.segments[1].end, 26);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 26);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_segments_with_higher_differences_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            44,
            [
                // only evaluating every secondth 422 pixel
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(1),
                row(1), // skipped
                row(2),
                row(2), // skipped
                row(3),
                row(3), // skipped
                row(4),
                row(4), // skipped
                row(5),
                // segment boundary will be here
                row(5), // skipped
                row(4),
                row(4), // skipped
                row(3),
                row(3), // skipped
                row(2),
                row(2), // skipped
                row(1),
                row(1), // skipped
                row(0),
                // segment boundary will be here
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped

                        // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        // y  y_median  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 1
        // 1  1          1     0         0                   0             0
        // 2
        // 2  2          1     1         1                   0             0
        // 3
        // 3  3          1     1         1                   0             0
        // 4
        // 4  4          1     1         1                   0             0
        // 5
        // 5  5          1     1         1                   0             0
        // 4
        // 4  4         -1     1         1                   1             1 -> end segment at position 17
        // 3
        // 3  3         -1    -1         0                   1             0
        // 2
        // 2  2         -1    -1         0                   1             0
        // 1
        // 1  1         -1    -1         0                   1             0
        // 0
        // 0  0         -1    -1         0                   1             0
        // 0
        // 0  0          0    -1         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0
        // -> end segment at position 44

        // y  y_median  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 1
        // 1  1          1     0         0                    1              0
        // 2
        // 2  2          1     1         0                    1              0
        // 3
        // 3  3          1     1         0                    1              0
        // 4
        // 4  4          1     1         0                    1              0
        // 5
        // 5  5          1     1         0                    1              0
        // 4
        // 4  4         -1     1         0                    0              0
        // 3
        // 3  3         -1    -1         1                    0              0
        // 2
        // 2  2         -1    -1         1                    0              0
        // 1
        // 1  1         -1    -1         1                    0              0
        // 0
        // 0  0         -1    -1         1                    0              0
        // 0
        // 0  0          0    -1         1                    1              1 -> end segment at position 27
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0
        // -> end segment at position 44

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 17);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 17);
        assert_eq!(scan_line.segments[1].end, 27);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 27);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_one_vertical_segment_with_increasing_differences_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            16,
            vec![
                // only evaluating every secondth 422 pixel
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                YCbCr422::new(6, 0, 0, 0),
                YCbCr422::new(6, 0, 0, 0), // skipped
                YCbCr422::new(10, 0, 0, 0),
                YCbCr422::new(10, 0, 0, 0), // skipped
                YCbCr422::new(15, 0, 0, 0),
                YCbCr422::new(15, 0, 0, 0), // skipped
                YCbCr422::new(21, 0, 0, 0),
                YCbCr422::new(21, 0, 0, 0), // skipped

                                            // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        //  y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        //  0  0     0          0                   1             0
        //  0  0     0          0                   1             0
        //  1  1     0          0                   0             0
        //  3  2     1          1                   0             0
        //  6  3     2          1                   0             0
        // 10  4     3          1                   0             0
        // 15  5     4          1                   0             0
        // 21  6     5          1                   0             0
        // -> end segment at position 16

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_one_vertical_segment_with_increasing_differences_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            16,
            [
                // only evaluating every secondth 422 pixel
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(1),
                row(1), // skipped
                row(3),
                row(3), // skipped
                row(6),
                row(6), // skipped
                row(10),
                row(10), // skipped
                row(15),
                row(15), // skipped
                row(21),
                row(21), // skipped
                         // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
        };

        //  y  y_median  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        //  0
        //  0   0        0     0          0                   1             0
        //  0
        //  0   0        0     0          0                   1             0
        //  1
        //  1   1        1     0          0                   0             0
        //  3
        //  3   3        2     1          1                   0             0
        //  6
        //  6   6        3     2          1                   0             0
        // 10
        // 10  10        4     3          1                   0             0
        // 15
        // 15  15        5     4          1                   0             0
        // 21
        // 21
        // -> end segment at position 16

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn median_of_three_with_same_values() {
        // first == second == third
        assert_eq!(median_of_three([0, 0, 0]), 0);
        // first < second == third
        assert_eq!(median_of_three([0, 1, 1]), 1);
        // first == second < third
        assert_eq!(median_of_three([0, 0, 1]), 0);
        // first == third < second
        assert_eq!(median_of_three([0, 1, 0]), 0);
    }

    #[test]
    fn median_of_three_with_different_values() {
        // first <= second <= third
        assert_eq!(median_of_three([0, 1, 2]), 1);
        // first <= third < second
        assert_eq!(median_of_three([0, 2, 1]), 1);
        // third < first <= second
        assert_eq!(median_of_three([1, 2, 0]), 1);
        // second < first <= third
        assert_eq!(median_of_three([1, 0, 2]), 1);
        // second <= third < first
        assert_eq!(median_of_three([2, 0, 1]), 1);
        // third < second <= first
        assert_eq!(median_of_three([2, 1, 0]), 1);
    }

    #[test]
    fn median_of_five_calculates_median() {
        for (first, second, third, fourth, fifth) in iproduct!(0..5, 0..5, 0..5, 0..5, 0..5) {
            let calculated_median = median_of_five([first, second, third, fourth, fifth]);
            let mut numbers = [first, second, third, fourth, fifth];
            numbers.sort();
            let real_median = numbers[2];
            assert_eq!(calculated_median,real_median, "test_case: {first} {second} {third} {fourth} {fifth}, median_of_five: {calculated_median}");
        }
    }
}
