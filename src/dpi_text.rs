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


/// Converts between the editor's logical text dimensions and the pixel
/// dimensions used to rasterize text.
///
/// Pötyi lays out its interface in display-independent logical units.
/// SDL_ttf, however, produces pixel surfaces. Rasterizing the font at this
/// scale and copying the resulting surface into the same logical destination
/// keeps the texture close to a one-to-one mapping with output pixels instead
/// of enlarging a low-resolution texture with SDL's logical presentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DpiTextMetrics {
    raster_scale: f32,
}

impl DpiTextMetrics {
    /// Construct metrics for an SDL display/output scale.
    ///
    /// SDL reports zero on failure. Non-finite and non-positive values also
    /// cannot describe a usable raster scale, so all of them fall back to 1x.
    pub(crate) fn new(
        raster_scale: f32,
    ) -> Self {
        Self {
            raster_scale:
                normalized_raster_scale(
                    raster_scale,
                ),
        }
    }

    /// The sanitized number of output pixels per logical unit.
    pub(crate) fn raster_scale(
        self,
    ) -> f32 {
        self.raster_scale
    }

    /// Convert a logical font size into the size SDL_ttf should rasterize.
    pub(crate) fn raster_font_size(
        self,
        logical_size: f32,
    ) -> f32 {
        logical_size * self.raster_scale
    }

    /// Whether a newly reported output scale is materially different.
    ///
    /// SDL_ttf clears cached glyphs when its font size changes. The relative
    /// tolerance prevents insignificant floating-point jitter from causing a
    /// cache flush on every redraw while remaining far below one physical
    /// pixel at Pötyi's font size.
    pub(crate) fn needs_update(
        self,
        raster_scale: f32,
    ) -> bool {
        let raster_scale =
            normalized_raster_scale(
                raster_scale,
            );

        let tolerance =
            0.001
                * self.raster_scale
                    .abs()
                    .max(
                        raster_scale.abs(),
                    )
                    .max(1.0);

        (
            self.raster_scale
                - raster_scale
        )
            .abs()
            > tolerance
    }
}

impl Default for DpiTextMetrics {
    fn default() -> Self {
        Self::new(1.0)
    }
}

fn normalized_raster_scale(
    raster_scale: f32,
) -> f32 {
    if raster_scale.is_finite()
        && raster_scale > 0.0
    {
        raster_scale
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::DpiTextMetrics;

    #[test]
    fn one_x_keeps_font_and_surface_dimensions() {
        let metrics =
            DpiTextMetrics::new(1.0);

        assert_eq!(
            metrics.raster_scale(),
            1.0,
        );

        assert_eq!(
            metrics.raster_font_size(18.0),
            18.0,
        );

    }

    #[test]
    fn two_x_uses_high_resolution_raster_dimensions() {
        let metrics =
            DpiTextMetrics::new(2.0);

        assert_eq!(
            metrics.raster_font_size(18.0),
            36.0,
        );

    }

    #[test]
    fn fractional_scale_adjusts_font_size() {
        let metrics =
            DpiTextMetrics::new(1.5);

        assert_eq!(
            metrics.raster_font_size(18.0),
            27.0,
        );

    }

    #[test]
    fn invalid_scales_fall_back_to_one_x() {
        for scale in [
            0.0,
            -1.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let metrics =
                DpiTextMetrics::new(scale);

            assert_eq!(
                metrics,
                DpiTextMetrics::default(),
            );
        }
    }

    #[test]
    fn update_check_ignores_tiny_scale_jitter() {
        let metrics =
            DpiTextMetrics::new(2.0);

        assert!(!metrics.needs_update(2.0));
        assert!(!metrics.needs_update(2.001));
        assert!(metrics.needs_update(2.01));
        assert!(metrics.needs_update(1.5));
    }

    #[test]
    fn update_check_normalizes_invalid_new_scale() {
        let one_x =
            DpiTextMetrics::new(1.0);

        let two_x =
            DpiTextMetrics::new(2.0);

        assert!(!one_x.needs_update(0.0));
        assert!(!one_x.needs_update(f32::NAN));
        assert!(two_x.needs_update(0.0));
    }
}
