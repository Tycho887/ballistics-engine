use anise::{
    almanac::metaload::MetaFile,
    constants::{
        celestial_objects::{MOON, SUN},
        frames::{EARTH_J2000, IAU_EARTH_FRAME},
    },
};
use hifitime::{Epoch, Unit};
use log::warn;
use nyx::{
    cosmic::{Mass, MetaAlmanac, Orbit, SRPData},
    dynamics::{Harmonics, OrbitalDynamics, SolarPressure, SpacecraftDynamics},
    io::{gravity::HarmonicsMem, ExportCfg},
    od::prelude::*,
    od::simulator::{Strand, TrackingArcSim, TrkConfig},
    od::msr::{MeasurementType, TrackingDataArc},    propagators::Propagator,
    Spacecraft, State,
};
use polars::{
    frame::column::ScalarColumn,
    prelude::{df, AnyValue, ChunkCompareIneq, Column, DataType, Scalar},
};

use std::{error::Error, sync::Arc};

struct TLE {
    name: String,
    line1: String,
    line2: String,
}

struct TLEData {
    name: String,
    epoch: Epoch,
    inclination: f64,
    raan: f64,
    eccentricity: f64,
    arg_perigee: f64,
    mean_anomaly: f64,
    mean_motion: f64,
    BSTAR: f64,
}

fn splitTLE(tle: &str) -> TLE {
    let lines: Vec<&str> = tle.lines().collect();
    if lines.len() != 3 {
        panic!("TLE must have exactly 3 lines");
    }
    TLE {
        name: lines[0].to_string(),
        line1: lines[1].to_string(),
        line2: lines[2].to_string(),
    }
}

fn parseTLE(tle: &TLE) -> TLEData {
    let line1 = &tle.line1;
    let line2 = &tle.line2;

    // Parse line 1
    let epoch_year = line1[18..20].parse::<u32>().unwrap();
    let epoch_day = line1[20..32].parse::<f64>().unwrap();
    let epoch = Epoch::from_gregorian_utc_hms(2000 + epoch_year, 1, 1, 0, 0, 0)
        + hifitime::Duration::from_seconds(epoch_day * 24.0 * 3600.0);

    // Parse line 2
    let inclination = line2[8..16].trim().parse::<f64>().unwrap();
    let raan = line2[17..25].trim().parse::<f64>().unwrap();
    let eccentricity = line2[26..33].trim().parse::<f64>().unwrap() / 1e7;
    let arg_perigee = line2[34..42].trim().parse::<f64>().unwrap();
    let mean_anomaly = line2[43..51].trim().parse::<f64>().unwrap();
    let mean_motion = line2[52..63].trim().parse::<f64>().unwrap();
    let bstar = line1[53..61].trim().parse::<f64>().unwrap();

    TLEData {
        name: tle.name.clone(),
        epoch,
        inclination,
        raan,
        eccentricity,
        arg_perigee,
        mean_anomaly,
        mean_motion,
        BSTAR: bstar,
    }
}

fn tle_to_orbit(tle_data: &TLEData, frame: &dyn anise::frames::Frame) -> Orbit {
    let mu = frame.mu();
    let a = (mu / (tle_data.mean_motion * 2.0 * std::f64::consts::PI / 86400.0).powi(2)).powf(1.0 / 3.0);
    Orbit::try_keplerian_altitude(
        a - frame.radius(), // altitude
        tle_data.eccentricity,
        tle_data.inclination,
        tle_data.raan,
        tle_data.arg_perigee,
        tle_data.mean_anomaly,
        tle_data.epoch,
        frame.clone(),
    ).unwrap()
}

fn getSpacecraft(orbit: Orbit) -> Spacecraft {
    // Let's build a cubesat sized spacecraft, with an SRP area of 10 cm^2 and a mass of 9.6 kg.
    Spacecraft::builder()
        .orbit(orbit)
        .mass(Mass::from_dry_mass(9.60))
        .srp(SRPData {
            area_m2: 10e-4,
            coeff_reflectivity: 1.1,
        })
        .build()
}