use rand::Rng;
use scan::ScanResult;
use station::StationId;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

type SpaceUnit = u32;
pub type SpaceCoord = (SpaceUnit, SpaceUnit, SpaceUnit);
type GalaxySector = (
    (SpaceUnit, SpaceUnit),
    (SpaceUnit, SpaceUnit),
    (SpaceUnit, SpaceUnit),
);

const SECTOR_SIZE: (SpaceUnit, SpaceUnit, SpaceUnit) = (1000, 1000, 1000);
const PLANETS_PER_SECTOR: usize = 10;

pub mod planet;
pub mod scan;
pub mod station;

#[allow(dead_code)]
pub enum SpaceObject {
    BaseStation(Arc<RwLock<station::Station>>),
    Planet(Arc<RwLock<planet::Planet>>),
}

struct GalaxyMap {
    objects: BTreeMap<SpaceUnit, BTreeMap<SpaceUnit, BTreeMap<SpaceUnit, SpaceObject>>>,
    discovered: Vec<GalaxySector>,
}

impl GalaxyMap {
    pub fn empty() -> GalaxyMap {
        GalaxyMap {
            objects: BTreeMap::new(),
            discovered: vec![],
        }
    }

    // X, Y and Z can be any point from the given sector
    pub fn generate_sector(&mut self, coord: &SpaceCoord) {
        let (x, y, z) = coord;
        let (secx, secy, secz) = compute_sector(*x, *y, *z);
        log::debug!(
            "Generating sector ({}-{}, {}-{}, {}-{})",
            secx.0,
            secx.1,
            secy.0,
            secy.1,
            secz.0,
            secz.1,
        );
        let mut rng = rand::rng();
        for _ in 0..PLANETS_PER_SECTOR {
            let x = rng.random_range(secx.0..secx.1);
            let y = rng.random_range(secy.0..secy.1);
            let z = rng.random_range(secz.0..secz.1);
            let planet = Arc::new(RwLock::new(planet::Planet::random((x, y, z), &mut rng)));
            if self
                .insert(&(x, y, z), SpaceObject::Planet(planet))
                .is_err()
            {
                continue;
            }
        }
    }

    pub fn is_discovered(&self, coord: &SpaceCoord) -> bool {
        let (x, y, z) = coord;
        for ((sx, ex), (sy, ey), (sz, ez)) in self.discovered.iter() {
            if (x < sx) || (x > ex) || (y < sy) || (y > ey) || (z < sz) || (z > ez) {
                continue;
            }
            return true;
        }
        false
    }

    pub fn get<'a>(&'a self, coord: &SpaceCoord) -> Option<&'a SpaceObject> {
        let (x, y, z) = coord;
        self.objects.get(x)?.get(y)?.get(z)
    }

    pub fn insert(&mut self, coord: &SpaceCoord, obj: SpaceObject) -> Result<(), ()> {
        let (x, y, z) = coord;
        if let Some(ref mut ydata) = self.objects.get_mut(x) {
            if let Some(ref mut zdata) = ydata.get_mut(y) {
                if zdata.get(z).is_some() {
                    return Err(());
                } else {
                    zdata.insert(*z, obj);
                }
            } else {
                let mut zdata = BTreeMap::new();
                zdata.insert(*z, obj);
                ydata.insert(*y, zdata);
            }
        } else {
            let mut zdata = BTreeMap::new();
            zdata.insert(*z, obj);
            let mut ydata = BTreeMap::new();
            ydata.insert(*y, zdata);
            self.objects.insert(*x, ydata);
        }
        Ok(())
    }

    fn list_objects_in_sector(&self, sector: GalaxySector) -> Vec<&SpaceObject> {
        let mut objects = vec![];
        let mut niter = 0;
        for (x, ydata) in self.objects.iter() {
            niter += 1;
            if (x < &sector.0 .0) || (x > &sector.0 .1) {
                continue;
            }
            for (y, zdata) in ydata.iter() {
                niter += 1;
                if (y < &sector.1 .0) || (y > &sector.1 .1) {
                    continue;
                }
                for (z, obj) in zdata.iter() {
                    niter += 1;
                    if (z < &sector.2 .0) || (z > &sector.2 .1) {
                        continue;
                    }
                    objects.push(obj);
                }
            }
        }
        log::debug!("Took {niter} iterations to find {} objects", objects.len());
        objects
    }
}

#[derive(Clone)]
pub struct Galaxy(Arc<RwLock<GalaxyMap>>);

impl Galaxy {
    pub fn init() -> Galaxy {
        Galaxy(Arc::new(RwLock::new(GalaxyMap::empty())))
    }

    // TODO (#11) Generate based on the galaxy
    pub fn init_new_station(&self) -> (StationId, SpaceCoord) {
        let mut rng = rand::rng();
        let coord = (rng.random(), rng.random(), rng.random());
        let id = rng.random();
        let station = Arc::new(RwLock::new(station::Station::init(id, coord)));

        let mut galaxy = self.0.write().unwrap();
        let res = galaxy.insert(&coord, SpaceObject::BaseStation(station));
        if res.is_err() {
            return self.init_new_station();
        }
        if !galaxy.is_discovered(&coord) {
            galaxy.generate_sector(&coord);
        }

        (id, coord)
    }

    pub fn get_station(&self, coord: &SpaceCoord) -> Option<Arc<RwLock<station::Station>>> {
        let galaxy = self.0.read().unwrap();
        let obj = galaxy.get(coord)?;
        let SpaceObject::BaseStation(station) = obj else {
            return None;
        };
        Some(station.clone())
    }

    pub fn scan_sector(&self, center: &SpaceCoord, strengh: f64) -> ScanResult {
        let mut results = ScanResult::empty();
        for sector in sectors_around(center, strengh) {
            for obj in self.0.read().unwrap().list_objects_in_sector(sector) {
                results.add(obj);
            }
        }
        results
    }
}

// TODO (#33)   Unit tests on this one
fn compute_sector(x: SpaceUnit, y: SpaceUnit, z: SpaceUnit) -> GalaxySector {
    let start_x = x - (x % SECTOR_SIZE.0);
    let end_x = start_x.saturating_add(SECTOR_SIZE.0);
    let start_y = y - (y % SECTOR_SIZE.1);
    let end_y = start_y.saturating_add(SECTOR_SIZE.1);
    let start_z = z - (z % SECTOR_SIZE.2);
    let end_z = start_z.saturating_add(SECTOR_SIZE.2);
    ((start_x, end_x), (start_y, end_y), (start_z, end_z))
}

pub fn translation(start: SpaceCoord, direction: (f64, f64, f64), dist: f64) -> SpaceCoord {
    (
        ((start.0 as f64) + (dist * direction.0)) as SpaceUnit,
        ((start.1 as f64) + (dist * direction.1)) as SpaceUnit,
        ((start.2 as f64) + (dist * direction.2)) as SpaceUnit,
    )
}

// TODO (#27)    Make this scan use a sphere from the center point
fn sectors_around(center: &SpaceCoord, radius: f64) -> Vec<GalaxySector> {
    let mut sectors = vec![];
    let centersec = compute_sector(center.0, center.1, center.2);

    let xsecstart = ((centersec.0 .0 as f64) - (radius * (SECTOR_SIZE.0 as f64))) as SpaceUnit;
    let nsector_x = (1.0 + (2.0 * radius * (SECTOR_SIZE.0 as f64))) as SpaceUnit;
    let xsecend = ((centersec.0 .1 as f64) + (radius * (SECTOR_SIZE.0 as f64))) as SpaceUnit;
    debug_assert_eq!(xsecstart + (nsector_x * SECTOR_SIZE.0), xsecend);

    let ysecstart = ((centersec.1 .0 as f64) - (radius * (SECTOR_SIZE.1 as f64))) as SpaceUnit;
    let nsector_y = (1.0 + (2.0 * radius * (SECTOR_SIZE.1 as f64))) as SpaceUnit;
    let ysecend = ((centersec.1 .1 as f64) + (radius * (SECTOR_SIZE.1 as f64))) as SpaceUnit;
    debug_assert_eq!(ysecstart + (nsector_y * SECTOR_SIZE.1), ysecend);

    let zsecstart = ((centersec.2 .0 as f64) - (radius * (SECTOR_SIZE.2 as f64))) as SpaceUnit;
    let nsector_z = (1.0 + (2.0 * radius * (SECTOR_SIZE.2 as f64))) as SpaceUnit;
    let zsecend = ((centersec.2 .1 as f64) + (radius * (SECTOR_SIZE.2 as f64))) as SpaceUnit;
    debug_assert_eq!(zsecstart + (nsector_z * SECTOR_SIZE.2), zsecend);

    for sx in 0..nsector_x {
        for sy in 0..nsector_y {
            for sz in 0..nsector_z {
                sectors.push((
                    (
                        xsecstart + (sx * SECTOR_SIZE.0),
                        xsecstart + ((sx + 1) * SECTOR_SIZE.0),
                    ),
                    (
                        ysecstart + (sy * SECTOR_SIZE.1),
                        ysecstart + ((sy + 1) * SECTOR_SIZE.1),
                    ),
                    (
                        zsecstart + (sz * SECTOR_SIZE.2),
                        zsecstart + ((sz + 1) * SECTOR_SIZE.2),
                    ),
                ))
            }
        }
    }

    sectors
}
