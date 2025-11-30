use crate::util::error::N3gbError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexagonDims {
    pub a: f64,
    pub r_circum: f64,
    pub r_apothem: f64,
    pub d_corners: f64,
    pub d_flats: f64,
    pub perimeter: f64,
    pub area: f64,
}

pub fn from_side(a: f64) -> Result<HexagonDims, N3gbError> {
    if a <= 0.0 {
        return Err(N3gbError::InvalidDimension(
            "Side length must be positive".to_string(),
        ));
    }

    let sqrt3 = 3.0_f64.sqrt();
    let r_circum = a;
    let r_apothem = (sqrt3 / 2.0) * a;
    let d_corners = 2.0 * a;
    let d_flats = sqrt3 * a;
    let perimeter = 6.0 * a;
    let area = (3.0 * sqrt3 / 2.0) * a * a;

    Ok(HexagonDims {
        a,
        r_circum,
        r_apothem,
        d_corners,
        d_flats,
        perimeter,
        area,
    })
}


pub fn from_circumradius(r: f64) -> Result<HexagonDims, N3gbError> {
    from_side(r)
}

pub fn from_apothem(r: f64) -> Result<HexagonDims, N3gbError> {
    if r <= 0.0 {
        return Err(N3gbError::InvalidDimension(
            "Apothem must be positive".to_string(),
        ));
    }

    let sqrt3 = 3.0_f64.sqrt();
    let a = 2.0 * r / sqrt3;
    from_side(a)
}

pub fn from_across_flats(df: f64) -> Result<HexagonDims, N3gbError> {
    if df <= 0.0 {
        return Err(N3gbError::InvalidDimension(
            "Across-flats must be positive".to_string(),
        ));
    }

    let sqrt3 = 3.0_f64.sqrt();
    let a = df / sqrt3;
    from_side(a)
}

pub fn from_across_corners(dc: f64) -> Result<HexagonDims, N3gbError> {
    if dc <= 0.0 {
        return Err(N3gbError::InvalidDimension(
            "Across-corners must be positive".to_string(),
        ));
    }

    let a = dc / 2.0;
    from_side(a)
}

pub fn from_area(area: f64) -> Result<HexagonDims, N3gbError> {
    if area <= 0.0 {
        return Err(N3gbError::InvalidDimension(
            "Area must be positive".to_string(),
        ));
    }

    let sqrt3 = 3.0_f64.sqrt();
    let a = ((2.0 * area) / (3.0 * sqrt3)).sqrt();
    from_side(a)
}

pub fn bounding_box(a: f64, pointy_top: bool) -> Result<(f64, f64), N3gbError> {
    if a <= 0.0 {
        return Err(N3gbError::InvalidDimension(
            "Side length must be positive".to_string(),
        ));
    }

    let sqrt3 = 3.0_f64.sqrt();
    let dc = 2.0 * a;
    let df = sqrt3 * a;

    if pointy_top {
        Ok((df, dc))
    } else {
        Ok((dc, df))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexagon_dimensions() -> Result<(), N3gbError> {
        let dims = from_side(10.0)?;

        assert!((dims.a - 10.0).abs() < 0.001);
        assert!((dims.r_circum - 10.0).abs() < 0.001);
        assert!((dims.d_corners - 20.0).abs() < 0.001);
        assert!((dims.perimeter - 60.0).abs() < 0.001);

        let dims2 = from_across_flats(dims.d_flats)?;
        assert!((dims2.a - 10.0).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_bounding_box() -> Result<(), N3gbError> {
        let (w, h) = bounding_box(10.0, true)?;
        assert!((w - 17.320508).abs() < 0.001); // sqrt(3) * 10
        assert!((h - 20.0).abs() < 0.001); // 2 * 10
        Ok(())
    }
}
