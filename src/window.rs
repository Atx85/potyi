// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.


use std::sync::{
    atomic::{
        AtomicI32,
        AtomicU32,
        Ordering,
    },
    Arc,
};

use sdl3::pixels::PixelFormat;
use sdl3::rect::Point;
use sdl3::surface::Surface;
use sdl3::video::{
    HitTestResult,
    Window,
    WindowBuilder,
    WindowPos,
};

const WINDOW_ICON_SIZE: u32 = 256;
static WINDOW_ICON_PIXELS: &[u8] =
    include_bytes!("../assets/potyi-icon-256.rgba");

/// Height of the custom title bar.
pub const TITLE_BAR_HEIGHT: i32 = 36;

/// Width of one custom window button.
pub const WINDOW_BUTTON_WIDTH: i32 = 46;

/// Total width occupied by:
///
///   -
///   □
///   X
///
/// Three buttons, each `WINDOW_BUTTON_WIDTH` wide.
pub const WINDOW_BUTTONS_WIDTH: i32 =
    WINDOW_BUTTON_WIDTH * 3;

fn normalized_scale(
    scale: f32,
) -> f32 {
    if scale.is_finite()
        && scale > 0.0
    {
        scale
    } else {
        1.0
    }
}

fn window_size_for_display_metrics(
    logical_width: u32,
    logical_height: u32,
    pixel_density: f32,
    display_scale: f32,
) -> (u32, u32) {
    let pixel_density =
        normalized_scale(
            pixel_density,
        );

    let display_scale =
        normalized_scale(
            display_scale,
        );

    let window_scale =
        display_scale / pixel_density;

    let width =
        (logical_width as f32 * window_scale)
            .round()
            .max(1.0) as u32;

    let height =
        (logical_height as f32 * window_scale)
            .round()
            .max(1.0) as u32;

    (width, height)
}

fn logical_render_size_for_display_scale(
    pixel_width: u32,
    pixel_height: u32,
    display_scale: f32,
) -> (u32, u32) {
    let display_scale =
        normalized_scale(
            display_scale,
        );

    let width =
        (pixel_width as f32 / display_scale)
            .round()
            .max(1.0) as u32;

    let height =
        (pixel_height as f32 / display_scale)
            .round()
            .max(1.0) as u32;

    (width, height)
}

fn window_coordinate_scale_for_display_metrics(
    pixel_density: f32,
    display_scale: f32,
) -> f32 {
    normalized_scale(
        pixel_density,
    ) / normalized_scale(
        display_scale,
    )
}

fn render_coordinate(
    window_coordinate: i32,
    window_coordinate_scale: f32,
) -> i32 {
    (
        window_coordinate as f32
            * normalized_scale(
                window_coordinate_scale,
            )
    )
        .round() as i32
}

pub fn logical_render_size(
    window: &Window,
) -> (u32, u32) {
    let pixel_size =
        window.size_in_pixels();

    logical_render_size_for_display_scale(
        pixel_size.0,
        pixel_size.1,
        window.display_scale(),
    )
}

pub fn window_coordinate_scale(
    window: &Window,
) -> f32 {
    window_coordinate_scale_for_display_metrics(
        window.pixel_density(),
        window.display_scale(),
    )
}

/// Shared state used by the SDL hit-test callback.
///
/// SDL's hit-test callback gives us the mouse position,
/// but the callback itself does not know the current
/// window width. Therefore the renderer updates this
/// value whenever the window is resized.
#[derive(Clone)]
pub struct WindowHitTestState {
    render_width: Arc<AtomicI32>,
    window_coordinate_scale: Arc<AtomicU32>,
}

impl WindowHitTestState {
    pub fn new(
        render_width: i32,
        window_coordinate_scale: f32,
    ) -> Self {
        Self {
            render_width: Arc::new(
                AtomicI32::new(render_width),
            ),
            window_coordinate_scale: Arc::new(
                AtomicU32::new(
                    normalized_scale(
                        window_coordinate_scale,
                    )
                        .to_bits(),
                ),
            ),
        }
    }

    pub fn set_metrics(
        &self,
        render_width: i32,
        window_coordinate_scale: f32,
    ) {
        self.render_width.store(
            render_width,
            Ordering::Relaxed,
        );

        self.window_coordinate_scale.store(
            normalized_scale(
                window_coordinate_scale,
            )
                .to_bits(),
            Ordering::Relaxed,
        );
    }

    fn render_width(&self) -> i32 {
        self.render_width.load(
            Ordering::Relaxed,
        )
    }

    fn window_coordinate_scale(&self) -> f32 {
        f32::from_bits(
            self.window_coordinate_scale.load(
                Ordering::Relaxed,
            ),
        )
    }
}

/// Create the Pötyi borderless window.
///
/// The native SDL title bar is removed.
///
/// We draw our own:
///
///     Pötyi                            - □ X
///
/// The operating system still handles:
///
/// - moving the window;
/// - resizing the window.
///
/// The custom buttons themselves are handled by
/// Pötyi's SDL mouse-event code.
///
/// `logical_width` and `logical_height` describe the desired content
/// size. SDL's live display scale and pixel density are used to convert
/// that size into the platform's native window coordinates.
pub fn create_window(
    video: &sdl3::VideoSubsystem,
    title: &str,
    logical_width: u32,
    logical_height: u32,
) -> Result<
    (Window, WindowHitTestState),
    String,
> {
    let mut builder: WindowBuilder =
        video.window(
            title,
            logical_width,
            logical_height,
        );

    // WindowBuilder methods mutate the builder
    // and return &mut WindowBuilder.
    builder
        .position_centered()
        .borderless()
        .resizable()
        .hidden();

    // Opt in to a Retina backing surface. Window size remains expressed
    // in native window coordinates; the extra pixels add detail only.
    #[cfg(target_os = "macos")]
    builder.high_pixel_density();

    let mut window =
        builder
            .build()
            .map_err(|e| e.to_string())?;

    let mut icon_pixels =
        WINDOW_ICON_PIXELS.to_vec();

    let icon =
        Surface::from_data(
            &mut icon_pixels,
            WINDOW_ICON_SIZE,
            WINDOW_ICON_SIZE,
            WINDOW_ICON_SIZE * 4,
            PixelFormat::RGBA32,
        )
        .map_err(|e| e.to_string())?;

    if !window.set_icon(&icon) {
        return Err(
            sdl3::get_error().to_string(),
        );
    }

    let initial_pixel_density =
        normalized_scale(
            window.pixel_density(),
        );

    let initial_display_scale =
        normalized_scale(
            window.display_scale(),
        );

    let window_size =
        window_size_for_display_metrics(
            logical_width,
            logical_height,
            initial_pixel_density,
            initial_display_scale,
        );

    window
        .set_size(
            window_size.0,
            window_size.1,
        )
        .map_err(|e| e.to_string())?;

    if !window.sync() {
        return Err(
            "Timed out while applying the window size"
                .to_string(),
        );
    }

    let pixel_density =
        normalized_scale(
            window.pixel_density(),
        );

    let display_scale =
        normalized_scale(
            window.display_scale(),
        );

    let final_window_size =
        window_size_for_display_metrics(
            logical_width,
            logical_height,
            pixel_density,
            display_scale,
        );

    if window.size() != final_window_size {
        window
            .set_size(
                final_window_size.0,
                final_window_size.1,
            )
            .map_err(|e| e.to_string())?;
    }

    if !window.set_position(
        WindowPos::Centered,
        WindowPos::Centered,
    ) {
        return Err(
            sdl3::get_error().to_string(),
        );
    }

    if !window.sync() {
        return Err(
            "Timed out while finalizing the window size"
                .to_string(),
        );
    }

    let render_width =
        logical_render_size(&window).0;

    let coordinate_scale =
        window_coordinate_scale(&window);

    let hit_test_state =
        WindowHitTestState::new(
            render_width as i32,
            coordinate_scale,
        );

    let state_for_callback =
        hit_test_state.clone();

    window
        .set_hit_test(
            move |point: Point| {
                let window_width =
                    state_for_callback
                        .render_width();

                let coordinate_scale =
                    state_for_callback
                        .window_coordinate_scale();

                let point_x =
                    render_coordinate(
                        point.x(),
                        coordinate_scale,
                    );

                let point_y =
                    render_coordinate(
                        point.y(),
                        coordinate_scale,
                    );

                let buttons_left =
                    window_width
                        - WINDOW_BUTTONS_WIDTH;

                /*
                 * The custom controls must NOT be draggable.
                 *
                 * Returning Normal here allows SDL to deliver
                 * the mouse click to our application, where the
                 * renderer/window-control code handles:
                 *
                 *     - minimize
                 *     □ maximize/restore
                 *     X close
                 */
                if point_y >= 0
                    && point_y < TITLE_BAR_HEIGHT
                    && point_x >= buttons_left
                {
                    return HitTestResult::Normal;
                }

                /*
                 * The rest of the custom title bar is draggable.
                 */
                if point_y >= 0
                    && point_y < TITLE_BAR_HEIGHT
                {
                    return HitTestResult::Draggable;
                }

                /*
                 * Everything below the title bar behaves
                 * normally, including the editor and resize
                 * borders.
                 */
                HitTestResult::Normal
            },
        )
        .map_err(|e| e.to_string())?;

    Ok((
        window,
        hit_test_state,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        logical_render_size_for_display_scale,
        render_coordinate,
        window_coordinate_scale_for_display_metrics,
        window_size_for_display_metrics,
        WINDOW_ICON_PIXELS,
        WINDOW_ICON_SIZE,
    };

    #[test]
    fn embedded_window_icon_has_rgba_pixel_data() {
        assert_eq!(
            WINDOW_ICON_PIXELS.len(),
            (WINDOW_ICON_SIZE * WINDOW_ICON_SIZE * 4)
                as usize,
        );
    }

    #[test]
    fn one_x_display_keeps_requested_size() {
        assert_eq!(
            window_size_for_display_metrics(
                800,
                600,
                1.0,
                1.0,
            ),
            (800, 600),
        );
    }

    #[test]
    fn retina_density_does_not_shrink_window() {
        assert_eq!(
            window_size_for_display_metrics(
                800,
                600,
                2.0,
                2.0,
            ),
            (800, 600),
        );
    }

    #[test]
    fn windows_content_scale_preserves_logical_size() {
        assert_eq!(
            window_size_for_display_metrics(
                800,
                600,
                1.0,
                2.0,
            ),
            (1600, 1200),
        );
    }

    #[test]
    fn fractional_metrics_are_rounded() {
        assert_eq!(
            window_size_for_display_metrics(
                800,
                600,
                2.0,
                1.5,
            ),
            (600, 450),
        );
    }

    #[test]
    fn invalid_metrics_fall_back_to_one_x() {
        assert_eq!(
            window_size_for_display_metrics(
                800,
                600,
                0.0,
                f32::NAN,
            ),
            (800, 600),
        );

        assert_eq!(
            logical_render_size_for_display_scale(
                1600,
                1200,
                f32::INFINITY,
            ),
            (1600, 1200),
        );
    }

    #[test]
    fn retina_backing_size_maps_to_logical_size() {
        assert_eq!(
            logical_render_size_for_display_scale(
                1600,
                1200,
                2.0,
            ),
            (800, 600),
        );
    }

    #[test]
    fn hit_test_coordinates_use_density_and_display_scale() {
        let retina_scale =
            window_coordinate_scale_for_display_metrics(
                2.0,
                2.0,
            );

        assert_eq!(
            render_coordinate(
                20,
                retina_scale,
            ),
            20,
        );

        let windows_two_x_scale =
            window_coordinate_scale_for_display_metrics(
                1.0,
                2.0,
            );

        assert_eq!(
            render_coordinate(
                20,
                windows_two_x_scale,
            ),
            10,
        );
    }
}
