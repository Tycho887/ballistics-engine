extern crate log;
extern crate nyx_space as nyx;
extern crate pretty_env_logger as pel;

use crate::noise::link_specific::ChipRate;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anise::{
    almanac::metaload::MetaFile,
    constants::frames::{EARTH_J2000, IAU_EARTH_FRAME},
    constants::celestial_objects::{ MOON, SUN},
};
use hifitime::{Epoch, Unit};
use nyx::{
    cosmic::{Mass, MetaAlmanac, Orbit, SRPData, Spacecraft, State},
    dynamics::{Harmonics, OrbitalDynamics, SolarPressure, SpacecraftDynamics},
    io::{gravity::HarmonicsMem, ExportCfg},
    od::prelude::*,
    od::simulator::{Strand, TrackingArcSim, TrkConfig},
    od::msr::{MeasurementType, TrackingDataArc},
    od::noise::link_specific::{CN0, SN0, CarrierFreq},
    propagators::Propagator,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Almanac & initial orbit (same as your example) ---
    let almanac = Arc::new(MetaAlmanac::latest().map_err(Box::new)?);
    let epoch = Epoch::from_gregorian_utc_hms(2024, 2, 29, 12, 13, 14);
    let earth_j2000 = almanac.frame_info(EARTH_J2000)?;

    let orbit = Orbit::try_keplerian_altitude(
        300.0, 0.015, 68.5, 65.2, 75.0, 0.0, epoch, earth_j2000,
    )?;

    let sc = Spacecraft::builder()
        .orbit(orbit)
        .mass(Mass::from_dry_mass(9.60))
        .srp(SRPData {
            area_m2: 10e-4,
            coeff_reflectivity: 1.1,
        })
        .build();

    // --- High-fidelity dynamics & propagation (same as your example) ---
    let mut orbital_dyn = OrbitalDynamics::point_masses(vec![MOON, SUN]);

    let mut jgm3_meta = MetaFile {
        uri: "http://public-data.nyxspace.com/nyx/models/JGM3.cof.gz".to_string(),
        crc32: Some(0xF446F027),
    };
    jgm3_meta.process(true)?;

    let harmonics_21x21 = Harmonics::from_stor(
        almanac.frame_info(IAU_EARTH_FRAME)?,
        HarmonicsMem::from_cof(&jgm3_meta.uri, 21, 21, true).unwrap(),
    );
    orbital_dyn.accel_models.push(harmonics_21x21);

    let srp_dyn = SolarPressure::default(EARTH_J2000, almanac.clone())?;
    let dynamics = SpacecraftDynamics::from_model(orbital_dyn, srp_dyn);

    let (_, trajectory) = Propagator::default(dynamics)
        .with(sc, almanac.clone())
        .until_epoch_with_traj(epoch + Unit::Day * 3)?;

    // ============================================================
    // 1. Configure a ground station with Range + Doppler (range-rate)
    // ============================================================
    let boulder_station = GroundStation::from_point(
        "Boulder, CO, USA".to_string(),
        40.014984,   // lat (deg)
        -105.270546, // lon (deg)
        1.6550,      // alt (km)
        almanac.frame_info(IAU_EARTH_FRAME)?,
    )
    // Add the measurement types you want to simulate.
    // Use appropriate stochastic noise for your hardware (examples below).
    .with_msr_type(
        MeasurementType::Range,
        StochasticNoise::from_hardware_range_km(
            1e-11,               // Allan deviation
            10.0.seconds(),
            ChipRate::StandardT4B,
            SN0::Average,
        ),
    )
    .with_msr_type(
        MeasurementType::Doppler,
        StochasticNoise::from_hardware_doppler_km_s(
            1e-11,               // Allan deviation
            10.0.seconds(),
            CarrierFreq::SBand,
            CN0::Average,
        ),
    );

    let mut devices = BTreeMap::new();
    devices.insert("Boulder, CO, USA".to_string(), boulder_station);

    // ============================================================
    // 2. Define tracking schedule (when to collect measurements)
    // ============================================================
    let mut configs = BTreeMap::new();
    configs.insert(
        "Boulder, CO, USA".to_string(),
        TrkConfig::builder()
            .strands(vec![Strand {
                start: epoch,
                end: epoch + Unit::Day * 3,
            }])
            .build(),
    );

    // ============================================================
    // 3. Simulate the tracking arc
    // ============================================================
    let mut trk = TrackingArcSim::<Spacecraft, GroundStation>::with_seed(
        devices,
        trajectory,
        configs,
        123, // RNG seed for reproducibility
    )?;
    trk.build_schedule(almanac.clone())?;
    let arc = trk.generate_measurements(almanac.clone())?;

    println!("Generated tracking arc: {}", arc);

    // ============================================================
    // 4. Export to CCSDS TDM v2.0
    // ============================================================
    // Aliases let you rename the ground station / spacecraft in the TDM file.
    let mut aliases = HashMap::new();
    aliases.insert("Boulder, CO, USA".to_string(), "Boulder".to_string());

    let tdm_path = arc.to_tdm_file(
        "CubeSat".to_string(),      // PARTICIPANT_2 (spacecraft) name in TDM
        Some(aliases),              // optional name remapping
        "./boulder_cubesat.tdm",    // output path
        ExportCfg::default(),
    )?;

    println!("TDM written to: {}", tdm_path.display());

    // ============================================================
    // 5. (Optional) Read it back
    // ============================================================
    let arc_rtn = TrackingDataArc::from_tdm(&tdm_path, None)?;
    println!("Read back from TDM: {}", arc_rtn);

    Ok(())
}   