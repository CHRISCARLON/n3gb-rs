use n3gb_rs::{HexCell, N3gbError};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Geometry {
    coordinates: Vec<Vec<Vec<f64>>>,
}

#[derive(Debug, Deserialize)]
struct GeoShape {
    geometry: Geometry,
}

#[derive(Debug, Deserialize)]
struct PipeData {
    geo_shape: GeoShape,
    asset_id: String,
}

fn main() -> Result<(), N3gbError> {
    let json_data = r#"[{"geo_point_2d": {"lon": -2.2479699500757597, "lat": 53.48082746395233}, "geo_shape": {"type": "Feature", "geometry": {"coordinates": [[[-2.246316659350537, 53.4833451912701], [-2.246462395007586, 53.4831297788124], [-2.246544226815247, 53.48300881400858], [-2.246657822483042, 53.48284090873182], [-2.246727838896523, 53.48273548227948], [-2.246834680538695, 53.48257506917826], [-2.246923790244199, 53.482444030971934], [-2.247069483965304, 53.482217283378446], [-2.247180586209335, 53.482046290709256], [-2.247251626467366, 53.48193857874573], [-2.24758920405991, 53.48143308739227], [-2.247643864861868, 53.481364508941425], [-2.247654076701864, 53.48135078934922], [-2.247789603032019, 53.48112600342602], [-2.24790823228074, 53.48094701267012], [-2.248034147730455, 53.48075411049558], [-2.248045274158281, 53.480737072226844], [-2.248047111113749, 53.48073425502267], [-2.248143577233219, 53.480586468558165], [-2.248143317873122, 53.48058540486364], [-2.248144670457642, 53.48058479533139], [-2.248178431642384, 53.480533072075545], [-2.248262219803749, 53.48041851972689], [-2.248400620989587, 53.480194248422386], [-2.248531408209862, 53.47999004605296], [-2.248709700845557, 53.47973132843837], [-2.24887817681766, 53.47942540563326], [-2.248905687789454, 53.47937860828217], [-2.249053476620937, 53.479127288299644], [-2.249105547510447, 53.47903935340452], [-2.249242721175016, 53.478807695109715], [-2.249360986690935, 53.47860994466818], [-2.249402262335218, 53.4785390203765], [-2.249427077133733, 53.47849514079153], [-2.249547467377941, 53.47835415684718], [-2.249586682592438, 53.478289106261386]]], "type": "MultiLineString"}, "properties": {}}, "type": "Main Pipe", "pressure": "LP", "material": "PE", "diameter": 500.0, "diam_unit": "MM", "carr_mat": null, "carr_dia": null, "carr_di_un": null, "asset_id": "CDT1274021173", "depth": null, "ag_ind": "False", "inst_date": "2009-09-22"}]"#;

    let pipes: Vec<PipeData> = serde_json::from_str(json_data).expect("valid JSON");
    let pipe = &pipes[0];

    let coords: Vec<(f64, f64)> = pipe.geo_shape.geometry.coordinates
        .iter()
        .flat_map(|line| line.iter())
        .map(|c| (c[0], c[1]))
        .collect();

    println!("Asset: {} ({} vertices)", pipe.asset_id, coords.len());

    let mut cells: HashMap<String, HexCell> = HashMap::new();
    for (lon, lat) in &coords {
        let cell = HexCell::from_wgs84(*lon, *lat, 12)?;
        cells.entry(cell.id.clone()).or_insert(cell);
    }

    println!("{:?}", cells);

    println!("Crosses {} hex cells at zoom 12:\n", cells.len());
    for cell in cells.values() {
        println!("  {} ({}, {})", cell.id, cell.row, cell.col);
    }

    Ok(())
}
