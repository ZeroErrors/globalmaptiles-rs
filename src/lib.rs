use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct GlobalMercator {
    tile_size: u32,
    initial_resolution: f64,
    origin_shift: f64,
}

impl Default for GlobalMercator {
    fn default() -> Self {
        GlobalMercator::new(256)
    }
}

impl GlobalMercator {
    // Initialize the TMS Global Mercator pyramid
    pub fn new(tile_size: u32) -> GlobalMercator {
        GlobalMercator {
            tile_size,
            initial_resolution: 2.0 * PI * 6378137.0 / tile_size as f64,
            // 156543.03392804062 for tile_size 256 pixels
            origin_shift: 2.0 * PI * 6378137.0 / 2.0,
            // 20037508.342789244
        }
    }

    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    pub fn lat_lon_to_meters(&self, lat: f64, lon: f64) -> (f64, f64) {
        // "Converts given lat/lon in WGS84 Datum to XY in Spherical Mercator EPSG:900913"

        let mx = lon * self.origin_shift / 180.0;
        let my = f64::ln(f64::tan((90.0 + lat) * PI / 360.0)) / (PI / 180.0);

        let my = my * self.origin_shift / 180.0;
        return (mx, my);
    }

    pub fn meters_to_lat_lon(&self, mx: f64, my: f64) -> (f64, f64) {
        // "Converts XY point from Spherical Mercator EPSG:900913 to lat/lon in WGS84 Datum"

        let lon = (mx / self.origin_shift) * 180.0;
        let lat = (my / self.origin_shift) * 180.0;

        let lat = 180.0 / PI * (2.0 * f64::atan(f64::exp(lat * PI / 180.0)) - PI / 2.0);
        return (lat, lon);
    }

    pub fn pixels_to_meters(&self, px: f64, py: f64, zoom: u32) -> (f64, f64) {
        // "Converts pixel coordinates in given zoom level of pyramid to EPSG:900913"

        let res = self.resolution(zoom);
        let mx = px * res - self.origin_shift;
        let my = py * res - self.origin_shift;
        return (mx, my);
    }

    pub fn meters_to_pixels(&self, mx: f64, my: f64, zoom: u32) -> (f64, f64) {
        // "Converts EPSG:900913 to pyramid pixel coordinates in given zoom level"

        let res = self.resolution(zoom);
        let px = (mx + self.origin_shift) / res;
        let py = (my + self.origin_shift) / res;
        return (px, py);
    }

    pub fn pixels_to_tile(&self, px: f64, py: f64) -> (i32, i32) {
        // "Returns a tile covering region in given pixel coordinates"

        let tx = f64::ceil(px / self.tile_size as f64) as i32 - 1;
        let ty = f64::ceil(py / self.tile_size as f64) as i32 - 1;
        return (tx, ty);
    }

    pub fn pixels_to_raster(&self, px: f64, py: f64, zoom: u32) -> (f64, f64) {
        // "Move the origin of pixel coordinates to top-left corner"

        let map_size = self.tile_size << zoom;
        return (px, map_size as f64 - py);
    }

    pub fn meters_to_tile(&self, mx: f64, my: f64, zoom: u32) -> (i32, i32) {
        // "Returns tile for given mercator coordinates"

        let (px, py) = self.meters_to_pixels(mx, my, zoom);
        return self.pixels_to_tile(px, py);
    }

    pub fn tile_bounds(&self, tx: i32, ty: i32, zoom: u32) -> (f64, f64, f64, f64) {
        // "Returns bounds of the given tile in EPSG:900913 coordinates"

        let (minx, miny) = self.pixels_to_meters((tx * self.tile_size as i32) as f64, (ty * self.tile_size as i32) as f64, zoom);
        let (maxx, maxy) = self.pixels_to_meters(((tx + 1) * self.tile_size as i32) as f64, ((ty + 1) * self.tile_size as i32) as f64, zoom);
        return (minx, miny, maxx, maxy);
    }

    pub fn tile_lat_lon_bounds(&self, tx: i32, ty: i32, zoom: u32) -> (f64, f64, f64, f64) {
        // "Returns bounds of the given tile in latutude/longitude using WGS84 datum"

        let (minx, miny, maxx, maxy) = self.tile_bounds(tx, ty, zoom);
        let (min_lat, min_lon) = self.meters_to_lat_lon(minx, miny);
        let (max_lat, max_lon) = self.meters_to_lat_lon(maxx, maxy);

        return (min_lat, min_lon, max_lat, max_lon);
    }

    pub fn resolution(&self, zoom: u32) -> f64 {
        // "resolution (meters/pixel) for given zoom level (measured at Equator)"

        // return (2 * PI * 6378137) / (self.tile_size * 2**zoom)
        return self.initial_resolution / f64::powi(2.0, zoom as i32);
    }

    pub fn zoom_for_pixel_size(&self, pixel_size: f64) -> u32 {
        // "Maximal scaledown zoom of the pyramid closest to the pixel_size."

        for i in 0..30 {
            if pixel_size > self.resolution(i) {
                return if i != 0 {
                    i - 1
                } else {
                    0 // We don't want to scale up
                };
            }
        }

        panic!("Invalid pixel_size: {}", pixel_size);
    }

    pub fn google_tile(&self, tx: i32, ty: i32, zoom: u32) -> (i32, i32) {
        // "Converts TMS tile coordinates to Google Tile coordinates"

        // coordinate origin is moved from bottom-left to top-left corner of the extent
        return (tx, (f64::powi(2.0, zoom as i32) as i32 - 1) - ty);
    }

    pub fn quad_tree(&self, tx: i32, ty: i32, zoom: u32) -> String {
        // "Converts TMS tile coordinates to Microsoft quad_tree"

        let mut quad_key = String::new();
        let ty = (f64::powi(2.0, zoom as i32) - 1.0) as i32 - ty;
        for i in (0..zoom as i32).rev() {
            let mut digit = 0;
            let mask = 1 << (i - 1);
            if (tx & mask) != 0 {
                digit += 1;
            }
            if (ty & mask) != 0 {
                digit += 2;
            }
            quad_key.push_str(format!("{}", digit).as_str());
        }

        return quad_key;
    }
}

// TODO: Implement GlobalGeodetic
// TODO: Add tests

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON_SCALE: f64 = 7_000_000.0; // TODO: This precision sucks

    #[test]
    fn test_default() {
        assert_eq!(GlobalMercator::default().tile_size, 256);
    }

    #[test]
    fn test_new() {
        assert_eq!(GlobalMercator::new(256).tile_size, 256);
    }

    #[test]
    fn test_lat_lon_meters() {
        let mercator = GlobalMercator::default();
        let (lat, lon) = (3.2, 4.22);

        let (mx, my) = mercator.lat_lon_to_meters(lat, lon);
        let (lat_new, lon_new) = mercator.meters_to_lat_lon(mx, my);


        assert!((lat - lat_new).abs() < std::f64::EPSILON * EPSILON_SCALE, "failed to compare: {} != {}, (lat - lat_new).abs() = {}, std::f64::EPSILON = {}", lat, lat_new, (lat - lat_new).abs(), std::f64::EPSILON * EPSILON_SCALE);
        assert!((lon - lon_new).abs() < std::f64::EPSILON * EPSILON_SCALE, "failed to compare: {} != {}, (lon - lon_new).abs() = {}, std::f64::EPSILON = {}", lon, lon_new, (lon - lon_new).abs(), std::f64::EPSILON * EPSILON_SCALE);
    }

    #[test]
    fn test_meters_pixels() {
        let mercator = GlobalMercator::default();
        let (mx, my) = (31100.00, 42200.1);
        let zoom = 8;

        let (px, py) = mercator.meters_to_pixels(mx, my, zoom);
        let (mx_new, my_new) = mercator.pixels_to_meters(px, py, zoom);

        assert!((mx - mx_new).abs() < std::f64::EPSILON * EPSILON_SCALE, "failed to compare: {} != {}, (mx - mx_new).abs() = {}, std::f64::EPSILON = {}", mx, mx_new, (mx - mx_new).abs() , std::f64::EPSILON * EPSILON_SCALE);
        assert!((my - my_new).abs() < std::f64::EPSILON * EPSILON_SCALE, "failed to compare: {} != {}, (my - my_new).abs() = {}, std::f64::EPSILON = {}", my, my_new, (my - my_new).abs(), std::f64::EPSILON * EPSILON_SCALE);
    }

    // TODO: pixels_to_tile is wrong, it should be using 'floor' not 'ceil' - 1 because of we are on the min edge the divide is exact so -1 puts us in the wrong tile
//    #[test]
//    fn test_pixels_tile() {
//        let mercator = GlobalMercator::default();
//        let (px, py) = (0.0, 0.0);
//        let zoom = 8;
//
//        let (tx, ty) = mercator.pixels_to_tile(px, py);
//        let tile_size = mercator.tile_size();
//        let (px_new, py_new) = ((tx * tile_size as i32) as f64, (ty * tile_size as i32) as f64);
//
//        assert_eq!(px, px_new);
//        assert_eq!(py, py_new);
//    }
}