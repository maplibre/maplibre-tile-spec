//! The coordinate reference systems of `CityGML` files, and their coordinates turned geodetic.
//!
//! Only the horizontal CRS is transformed: heights stay in the source's vertical datum, since a
//! mesh stores heights above its lowest vertex.

use anyhow::{Result as AnyResult, anyhow, bail};
use proj4rs::Proj;
use proj4rs::transform::{Transform, TransformClosure, transform};

/// The horizontal EPSG code a GML `srsName` names: `EPSG:25832`, `urn:ogc:def:crs:EPSG::25832`,
/// `http://www.opengis.net/def/crs/EPSG/0/25832`, a compound of those, or the German `AdV`'s
/// `urn:adv:crs:ETRS89_UTM32*DE_DHHN2016_NH`.
#[must_use]
pub fn epsg_of_srs_name(name: &str) -> Option<u16> {
    let lower = name.to_ascii_lowercase();
    if let Some(at) = lower.find("epsg") {
        let segment = lower[at + 4..].split([',', '+', '*']).next()?;
        let code = segment.split([':', '/', '#']).rfind(|s| !s.is_empty())?;
        return code.trim().parse().ok().map(|code| match code {
            // JGD2011 + JGD2011 (vertical) height, as PLATEAU names it
            6697 => 6668,
            // ETRS89 / UTM zone 32N or 33N + DHHN2016 height
            5555 => 25832,
            5556 => 25833,
            other => other,
        });
    }
    let at = lower.find("etrs89_utm")?;
    let zone: String = lower[at + 10..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    zone.parse::<u16>()
        .ok()
        .filter(|zone| (1..=60).contains(zone))
        .map(|zone| 25800 + zone)
}

/// Positions of one CRS to longitude and latitude in radians on WGS84, heights unchanged.
pub struct Reprojector {
    epsg: u16,
    src: Proj,
    dst: Proj,
    /// Whether the CRS is geographic, its coordinates latitude and longitude in degrees.
    pub geographic: bool,
}

impl Reprojector {
    pub fn new(epsg: u16) -> AnyResult<Self> {
        let src = Proj::from_epsg_code(epsg)
            .map_err(|e| anyhow!("EPSG:{epsg} is not a CRS this build knows: {e}"))?;
        let dst = Proj::from_proj_string("+proj=longlat +datum=WGS84 +no_defs")
            .map_err(|e| anyhow!("{e}"))?;
        Ok(Self {
            epsg,
            geographic: src.is_latlong(),
            src,
            dst,
        })
    }

    /// Replaces each of `points` by `[lon, lat, height]` in radians and the source's height unit.
    pub fn to_geodetic(&self, points: &mut [[f64; 3]]) -> AnyResult<()> {
        if self.geographic {
            for p in points.iter_mut() {
                *p = [p[1].to_radians(), p[0].to_radians(), p[2]];
            }
        }
        transform(&self.src, &self.dst, &mut Points(points))
            .map_err(|e| anyhow!("reprojecting from EPSG:{}: {e}", self.epsg))?;
        if points.iter().any(|p| p.iter().any(|v| !v.is_finite())) {
            bail!("a position does not reproject from EPSG:{}", self.epsg);
        }
        Ok(())
    }
}

struct Points<'a>(&'a mut [[f64; 3]]);

impl Transform for Points<'_> {
    fn transform_coordinates<F: TransformClosure>(
        &mut self,
        f: &mut F,
    ) -> proj4rs::errors::Result<()> {
        for p in self.0.iter_mut() {
            let (x, y, _) = f(p[0], p[1], p[2])?;
            *p = [x, y, p[2]];
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srs_names_name_their_horizontal_epsg_code() {
        for (name, code) in [
            ("EPSG:25832", 25832),
            ("urn:ogc:def:crs:EPSG::25833", 25833),
            ("http://www.opengis.net/def/crs/EPSG/0/6697", 6668),
            ("urn:ogc:def:crs,crs:EPSG::25832,crs:EPSG::7837", 25832),
            ("EPSG:5555", 25832),
            ("urn:adv:crs:ETRS89_UTM32*DE_DHHN2016_NH", 25832),
        ] {
            assert_eq!(epsg_of_srs_name(name), Some(code), "{name}");
        }
        assert_eq!(epsg_of_srs_name("local"), None);
        assert_eq!(epsg_of_srs_name("EPSG:"), None);
        assert_eq!(epsg_of_srs_name("urn:adv:crs:ETRS89_UTM60000"), None);
        assert_eq!(epsg_of_srs_name("urn:adv:crs:ETRS89_UTM0"), None);
    }

    #[test]
    fn geographic_positions_are_read_latitude_first() {
        let r = Reprojector::new(4326).unwrap();
        assert!(r.geographic);
        let mut points = [[35.66, 139.7, 40.0]];
        r.to_geodetic(&mut points).unwrap();
        assert!((points[0][0].to_degrees() - 139.7).abs() < 1e-9);
        assert!((points[0][1].to_degrees() - 35.66).abs() < 1e-9);
        assert!((points[0][2] - 40.0).abs() < 1e-9);
    }

    #[test]
    fn an_unknown_code_fails_with_its_name() {
        let err = Reprojector::new(1).err().unwrap();
        assert!(err.to_string().starts_with("EPSG:1 is not a CRS"), "{err}");
    }
}
