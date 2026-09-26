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
        AtomicBool,
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
/// window dimensions or maximized state. The renderer refreshes these metrics
/// on resize, display changes and maximize/restore events.
#[derive(Clone)]
pub struct WindowHitTestState {
    render_width: Arc<AtomicI32>,
    render_height: Arc<AtomicI32>,
    resize_enabled: Arc<AtomicBool>,
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
            render_height: Arc::new(AtomicI32::new(0)),
            resize_enabled: Arc::new(AtomicBool::new(false)),
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
        render_height: i32,
        window_coordinate_scale: f32,
        resize_enabled: bool,
    ) {
        self.render_height.store(render_height, Ordering::Relaxed);
        self.resize_enabled.store(resize_enabled, Ordering::Relaxed);
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

    fn hit_test(&self, point: Point) -> HitTestResult {
        let scale = self.window_coordinate_scale();
        frame_hit_test(
            render_coordinate(point.x(), scale),
            render_coordinate(point.y(), scale),
            self.render_width.load(Ordering::Relaxed),
            self.render_height.load(Ordering::Relaxed),
            self.resize_enabled.load(Ordering::Relaxed),
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

/// Borderless windows need explicit edge hit tests for native resizing.
/// Coordinates are display-scaled logical pixels, independent of editor zoom.
pub(crate) fn frame_hit_test(x: i32, y: i32, width: i32, height: i32, resize: bool) -> HitTestResult {
    use HitTestResult::*;
    if x < 0 || y < 0 || x >= width || y >= height { return Normal; }
    if resize {
        const EDGE: i32 = 6;
        const CORNER: i32 = 12;
        let left = x < EDGE;
        let right = x >= width - EDGE;
        let top = y < EDGE;
        let bottom = y >= height - EDGE;
        if (left && y < CORNER) || (top && x < CORNER) { return ResizeTopLeft; }
        if (right && y < CORNER) || (top && x >= width - CORNER) { return ResizeTopRight; }
        if (left && y >= height - CORNER) || (bottom && x < CORNER) { return ResizeBottomLeft; }
        if (right && y >= height - CORNER) || (bottom && x >= width - CORNER) { return ResizeBottomRight; }
        if left { return ResizeLeft; }
        if right { return ResizeRight; }
        if top { return ResizeTop; }
        if bottom { return ResizeBottom; }
    }
    if y < TITLE_BAR_HEIGHT && x < width - WINDOW_BUTTONS_WIDTH { Draggable } else { Normal }
}

pub(crate) fn can_resize(window: &Window) -> bool {
    !window.is_maximized() && window.fullscreen_state() == sdl3::video::FullscreenType::Off
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

    // Wayland compositors may not implement xdg-toplevel-icon-v1. The
    // desktop launcher can supply the icon; this must not prevent startup.
    if !window.set_icon(&icon) {
        sdl3::clear_error();
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

    // Wayland owns the placement of normal windows and rejects explicit
    // positioning. Centering is only a preference on other backends, too.
    if !window.set_position(
        WindowPos::Centered,
        WindowPos::Centered,
    ) {
        sdl3::clear_error();
    }

    if !window.sync() {
        return Err(
            "Timed out while finalizing the window size"
                .to_string(),
        );
    }

    let (render_width, render_height) = logical_render_size(&window);

    let coordinate_scale =
        window_coordinate_scale(&window);

    let hit_test_state =
        WindowHitTestState::new(
            render_width as i32,
            coordinate_scale,
        );

    hit_test_state.set_metrics(render_width as i32, render_height as i32, coordinate_scale,
        can_resize(&window));

    let state_for_callback =
        hit_test_state.clone();

    window
        .set_hit_test(
            move |point: Point| state_for_callback.hit_test(point),
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

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "Requires a Wayland compositor; run with SDL_VIDEODRIVER=wayland"]
    fn wayland_window_startup_smoke() {
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let driver = video.current_video_driver();
        assert_eq!(driver, "wayland", "this regression must exercise the Wayland backend");
        let (window, _hit_test) = super::create_window(&video, "Pötyi startup test", 800, 600)
            .unwrap_or_else(|e| panic!("{driver} window startup failed: {e}"));
        let mut canvas = sdl3::render::create_renderer(window, None).unwrap();
        assert!(canvas.window_mut().show(), "{driver}: {}", sdl3::get_error());
        canvas.set_draw_color(sdl3::pixels::Color::RGB(30, 30, 30));
        canvas.clear();
        canvas.present(); // Wayland needs the first frame to map the window.
        let mut events = sdl.event_pump().unwrap();
        for _ in events.poll_iter() {}
        let (width, height) = canvas.window().size_in_pixels();
        assert!(width > 0 && height > 0);
    }

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

    #[test]
    fn borderless_hit_tests_cover_edges_corners_title_and_controls() {
        use super::frame_hit_test;
        use sdl3::video::HitTestResult::*;
        for (x, y, expected) in [
            (0, 300, ResizeLeft), (799, 300, ResizeRight),
            (400, 0, ResizeTop), (400, 599, ResizeBottom),
            (0, 0, ResizeTopLeft), (799, 0, ResizeTopRight),
            (0, 599, ResizeBottomLeft), (799, 599, ResizeBottomRight),
            (10, 2, ResizeTopLeft), (797, 10, ResizeTopRight),
            (10, 598, ResizeBottomLeft), (797, 590, ResizeBottomRight),
            (20, 20, Draggable), (680, 20, Normal), (775, 20, Normal),
            (12, 44, Normal), (400, 300, Normal),
            (-1, 50, Normal), (800, 50, Normal), (50, 600, Normal),
        ] {
            assert_eq!(frame_hit_test(x, y, 800, 600, true), expected, "{x},{y}");
        }
        assert_eq!(frame_hit_test(5, 300, 800, 600, true), ResizeLeft);
        assert_eq!(frame_hit_test(6, 300, 800, 600, true), Normal);
    }

    #[test]
    fn resize_hit_tests_track_dimensions_scale_and_maximize_state() {
        use super::WindowHitTestState;
        use sdl3::{rect::Point, video::HitTestResult::*};
        let state = WindowHitTestState::new(800, 1.0);
        let callback = state.clone();
        state.set_metrics(800, 600, 0.5, true);
        assert_eq!(callback.hit_test(Point::new(1596, 600)), ResizeRight);
        assert_eq!(callback.hit_test(Point::new(800, 1196)), ResizeBottom);
        assert_eq!(callback.hit_test(Point::new(800, 80)), Normal);
        state.set_metrics(1000, 700, 1.0, true);
        assert_eq!(callback.hit_test(Point::new(798, 300)), Normal);
        assert_eq!(callback.hit_test(Point::new(998, 698)), ResizeBottomRight);
        state.set_metrics(1000, 700, 1.0, false);
        assert_eq!(callback.hit_test(Point::new(998, 698)), Normal);
        assert_eq!(callback.hit_test(Point::new(998, 2)), Normal);
        assert_eq!(callback.hit_test(Point::new(200, 2)), Draggable);
    }
}
