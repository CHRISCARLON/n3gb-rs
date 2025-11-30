use geo_types::{Coord, LineString, Point, Polygon};

pub fn create_hexagon(center_x: f64, center_y: f64, size: f64) -> Polygon<f64> {
    let mut coords = Vec::with_capacity(7);

    for i in 0..6 {
        let angle_deg = 30.0 + (i as f64 * 60.0);
        let angle_rad = angle_deg.to_radians();
        let x = center_x + size * angle_rad.cos();
        let y = center_y + size * angle_rad.sin();
        coords.push(Coord { x, y });
    }
    coords.push(coords[0]);

    Polygon::new(LineString::from(coords), vec![])
}

pub fn create_hexagon_from_point(center: &Point<f64>, size: f64) -> Polygon<f64> {
    create_hexagon(center.x(), center.y(), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::point;

    #[test]
    fn test_create_hexagon() {
        let hex = create_hexagon(100.0, 100.0, 10.0);
        let exterior = hex.exterior();
        assert_eq!(exterior.coords().count(), 7); // 6 vertices + 1 to close
        assert_eq!(exterior.0[0], exterior.0[6]); // First and last are same
    }

    #[test]
    fn test_create_hexagon_from_point() {
        let center = point! { x: 100.0, y: 100.0 };
        let hex = create_hexagon_from_point(&center, 10.0);
        let exterior = hex.exterior();
        assert_eq!(exterior.coords().count(), 7);
    }
}
